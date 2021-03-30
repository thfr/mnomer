use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, SampleRate, Stream,
};
use cpal::{SampleFormat, StreamConfig};

use crate::audiosignal::{settings, AudioSignal};
use std::{convert::TryFrom, sync::Mutex};

/// Metronome beat pattern types
#[derive(Debug, PartialEq, Clone)]
pub enum BeatPatternType {
    Accent,
    Beat,
    Pause,
}

impl TryFrom<&char> for BeatPatternType {
    type Error = String;

    fn try_from(value: &char) -> Result<Self, Self::Error> {
        match value {
            '!' => Ok(BeatPatternType::Accent),
            '+' => Ok(BeatPatternType::Beat),
            '.' => Ok(BeatPatternType::Pause),
            // anything else is an error
            x => Err(format!("char \"{}\" is not an BeatPatternType", x)),
        }
    }
}

/// Metronome beat pattern
#[derive(Debug, Clone)]
pub struct BeatPattern(pub Vec<BeatPatternType>);

impl TryFrom<&str> for BeatPattern {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = BeatPattern(Vec::with_capacity(value.len()));
        for element in value.chars() {
            result.0.push(BeatPatternType::try_from(&element)?);
        }
        Ok(result)
    }
}

/// A metronome sound player that realizes the beat playback
// #[derive(Debug)]
pub struct BeatPlayer {
    pub bpm: u16,
    pub beat: AudioSignal,
    pub ac_beat: AudioSignal,
    pub pattern: BeatPattern,
    stream: Option<Stream>,
    start_stop_mtx: Mutex<()>,
}

impl ToString for BeatPlayer {
    fn to_string(&self) -> String {
        format!(
            "bpm: {}, pattern: {:?}, playing: {}",
            self.bpm,
            self.pattern,
            self.is_playing()
        )
    }
}

impl BeatPlayer {
    pub fn new(
        bpm: u16,
        beat: AudioSignal,
        ac_beat: AudioSignal,
        pattern: BeatPattern,
    ) -> BeatPlayer {
        BeatPlayer {
            bpm,
            beat,
            ac_beat,
            pattern,
            stream: None,
            start_stop_mtx: Mutex::new(()),
        }
    }

    /// Check whether the beat playback is running or starting
    pub fn is_playing(&self) -> bool {
        let _lockguard = self.start_stop_mtx.try_lock();
        self.stream.is_some()
    }

    /// Stop the beat playback
    pub fn stop(&mut self) {
        let _mutex_guard = self
            .start_stop_mtx
            .lock()
            .expect("Playback start mutex is poisoned, aborting");
        match self.stream.as_mut() {
            Some(x) => x.pause().expect("Error during pause"),
            None => (),
        };
        self.stream = None;
    }

    /// Set the beat pattern
    ///
    /// Stops and resumes playback if playback is running
    pub fn set_pattern(&mut self, pattern: BeatPattern) -> Result<(), String> {
        if pattern.0.is_empty() {
            return Err("Beat pattern is empty, will not change anything".to_string());
        }
        let restart = if self.is_playing() {
            self.stop();
            true
        } else {
            false
        };

        let previous_pattern = pattern.0.clone();
        self.pattern.0 = pattern.0;

        if restart {
            match self.play_beat() {
                Err(_) => {
                    self.pattern.0 = previous_pattern;
                    return Err(
                        "New pattern does not seem to work, returning to previous pattern"
                            .to_string(),
                    );
                }
                _ => (),
            };
        }

        Ok(())
    }

    /// Set the beats per minute
    ///
    /// Stops and resumes playback if playback is running
    pub fn set_bpm(&mut self, bpm: u16) -> bool {
        if bpm == 0 {
            return false;
        }

        let restart = if self.is_playing() {
            self.stop();
            true
        } else {
            false
        };

        let previous_bpm = self.bpm;
        self.bpm = bpm;

        if restart {
            match self.play_beat() {
                Err(_) => {
                    self.bpm = previous_bpm;
                    return false;
                }
                _ => (),
            };
        }

        true
    }

    fn _fill_playback_buffer(&self) -> Result<AudioSignal, &'static str> {
        // Create the playback buffer over which the output loops
        // Use self.beat and silence to fill the buffer
        if self.beat.signal.is_empty() {
            return Err("No beat to play");
        }

        let samples_per_beat = ((60.0 * settings::SAMPLERATE) / self.bpm as f64).round() as isize;

        let silence_samples = samples_per_beat as isize - self.beat.signal.len() as isize;
        if silence_samples < 0 {
            return Err("Beat to long to play at current bpm");
        }

        let ac_silence_samples = samples_per_beat as isize - self.ac_beat.signal.len() as isize;
        if ac_silence_samples < 0 {
            return Err("Accentuated beat to long to play at current bpm");
        }

        // prepare the playback buffer
        let (ac_beat_count, beat_count, pause_count) = {
            let mut a = 0;
            let mut b = 0;
            let mut c = 0;
            for bpt in &self.pattern.0 {
                match bpt {
                    BeatPatternType::Accent => a += 1,
                    BeatPatternType::Beat => b += 1,
                    BeatPatternType::Pause => c += 1,
                }
            }
            (a, b, c)
        };
        let playback_buffer_samples = ac_beat_count
            * (self.ac_beat.signal.len() + ac_silence_samples as usize)
            + beat_count * (self.beat.signal.len() + silence_samples as usize)
            + pause_count * (samples_per_beat as usize);

        let mut playback_buffer = AudioSignal {
            signal: Vec::with_capacity(playback_buffer_samples),
            index: 0,
        };
        for beat_type in &self.pattern.0 {
            match beat_type {
                BeatPatternType::Accent => {
                    playback_buffer
                        .signal
                        .extend_from_slice(&self.ac_beat.signal[0..]);
                    for _ in 0..ac_silence_samples {
                        playback_buffer.signal.push(0);
                    }
                }
                BeatPatternType::Beat => {
                    playback_buffer
                        .signal
                        .extend_from_slice(&self.beat.signal[0..]);
                    for _ in 0..silence_samples {
                        playback_buffer.signal.push(0);
                    }
                }
                BeatPatternType::Pause => {
                    for _ in 0..samples_per_beat {
                        playback_buffer.signal.push(0);
                    }
                }
            }
        }
        Ok(playback_buffer)
    }

    pub fn play_beat(&mut self) -> Result<(), &str> {
        let lockguard = self.start_stop_mtx.try_lock();

        if lockguard.is_err() {
            return Err("Cannot start beat playback, it is already running");
        }

        let mut playback_buffer = match self._fill_playback_buffer() {
            Ok(audio_signal) => audio_signal,
            Err(msg) => return Err(msg),
        };

        let samples_callback = move |data: &mut [i16], _: &OutputCallbackInfo| {
            for sample in data.iter_mut() {
                *sample = playback_buffer.get_next_sample();
            }
        };
        self.stream = match init_audio_cpal(samples_callback) {
            Ok(x) => Some(x),
            Err(y) => return Err(y),
        };

        match self.stream.as_mut().unwrap().play() {
            Ok(_) => (),
            Err(_) => return Err("Something went wrong with beat playback"),
        };

        // everything was fine fine
        Ok(())
    }
}

fn init_audio_cpal<T>(samples_callback: T) -> Result<Stream, &'static str>
where
    T: FnMut(&mut [i16], &OutputCallbackInfo) + Send + 'static,
{
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    let mut config_option = None;
    for conf in device.supported_output_configs().unwrap() {
        if conf.sample_format() == SampleFormat::I16 {
            config_option = Some(conf.with_sample_rate(SampleRate(settings::SAMPLERATE as u32)));
            break;
        }
    }
    let config = match config_option {
        Some(x) => x,
        None => return Err("Could not find audio configuration of 48 kHz sample rate and audio sample format int 16"),
    };

    let config: StreamConfig = config.into();
    Ok(device
        .build_output_stream(&config, samples_callback, err_fn)
        .unwrap())
}
