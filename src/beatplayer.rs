use cpal::{SampleRate, Stream, SupportedStreamConfigRange, traits::{DeviceTrait, HostTrait, StreamTrait}};
use cpal::{BufferSize, SampleFormat, StreamConfig};

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
    pub beat: AudioSignal<f32>,
    pub ac_beat: AudioSignal<f32>,
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
        beat: AudioSignal<f32>,
        ac_beat: AudioSignal<f32>,
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

    fn _fill_playback_buffer(&self) -> Result<AudioSignal<f32>, &'static str> {
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
                        playback_buffer.signal.push(0f32);
                    }
                }
                BeatPatternType::Beat => {
                    playback_buffer
                        .signal
                        .extend_from_slice(&self.beat.signal[0..]);
                    for _ in 0..silence_samples {
                        playback_buffer.signal.push(0f32);
                    }
                }
                BeatPatternType::Pause => {
                    for _ in 0..samples_per_beat {
                        playback_buffer.signal.push(0f32);
                    }
                }
            }
        }
        Ok(playback_buffer)
    }

    pub fn play_beat(&mut self) -> Result<(), String> {
        let lockguard = self.start_stop_mtx.try_lock();

        if lockguard.is_err() {
            return Err("Cannot start beat playback, it is already running".into());
        }

        let playback_buffer = match self._fill_playback_buffer() {
            Ok(audio_signal) => audio_signal,
            Err(msg) => return Err(msg.into()),
        };

        self.stream = match init_audio_cpal(playback_buffer) {
            Ok(x) => Some(x),
            Err(y) => return Err(y),
        };

        match self.stream.as_mut().unwrap().play() {
            Ok(_) => (),
            Err(_) => return Err("Something went wrong with beat playback".into()),
        };

        // everything was fine fine
        Ok(())
    }
}

fn init_audio_cpal(playback_buffer: AudioSignal<f32>) -> Result<Stream, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);

    let supported_output_configs = match device.supported_output_configs() {
        Ok(x) => x.collect::<Vec<SupportedStreamConfigRange>>(),
        Err(error) => {
            return Err(format!(
                "Default audio output device has no supported configurations: {:?}",
                error
            ))
        }
    };

    // TODO: support other samplerates than 48kHz
    let supported_sample_types: Vec<SampleFormat> = supported_output_configs.iter()
        .filter(|x| {
            x.min_sample_rate().0 as f64 <= settings::SAMPLERATE
                && settings::SAMPLERATE <= x.max_sample_rate().0 as f64
        })
        .map(|x| x.sample_format())
        .collect();

    if supported_sample_types.is_empty() {
        let mut supported_configurations_str = String::new();
        for config in supported_output_configs.iter() {
            supported_configurations_str += format!("{:?}\n", config).as_str();
        }
        return Err(format!(
            "Default audio device not supported: only following configurations are supported\n{}", supported_configurations_str
        ));
    }

    let sampletype = if supported_sample_types.contains(&SampleFormat::F32) {
        SampleFormat::F32
    } else if supported_sample_types.contains(&SampleFormat::I16) {
        SampleFormat::I16
    } else {
        SampleFormat::U16
    };

    let my_config = StreamConfig {
        channels: 1,
        sample_rate: SampleRate(settings::SAMPLERATE as u32),
        buffer_size: BufferSize::Default,
    };

    //TODO: unify these lambdas somehow
    let stream = match sampletype {
        SampleFormat::F32 => {
            let mut playback_buffer: AudioSignal<f32> = playback_buffer.into();
            device.build_output_stream(
                &my_config,
                move |data, _| {
                    for sample in data.iter_mut() {
                        *sample = playback_buffer.get_next_sample();
                    }
                },
                err_fn,
            )
        }
        SampleFormat::I16 => {
            let mut playback_buffer: AudioSignal<i16> = playback_buffer.into();
            device.build_output_stream(
                &my_config,
                move |data, _| {
                    for sample in data.iter_mut() {
                        *sample = playback_buffer.get_next_sample();
                    }
                },
                err_fn,
            )
        }
        SampleFormat::U16 => {
            let mut playback_buffer: AudioSignal<u16> = playback_buffer.into();
            device.build_output_stream(
                &my_config,
                move |data, _| {
                    for sample in data.iter_mut() {
                        *sample = playback_buffer.get_next_sample();
                    }
                },
                err_fn,
            )
        }
    };

    match stream {
        Ok(stream) => Ok(stream),
        Err(x) => Err(format!(
            "Streamconfig {:?} is not supported, got error: {:?}",
            my_config, x
        )),
    }
}
