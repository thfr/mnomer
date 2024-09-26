use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};

use crate::{
    audiosignal::{samples_to_time, AudioSignal, ToneConfiguration},
    repl::repl::ReplApp,
};
use std::{
    convert::TryFrom,
    f64,
    fmt::Display,
    sync::Mutex,
    time::{Duration, Instant},
};

use crossterm::style::Attribute;

pub const BASE_BEAT_VALUE: u16 = 4;

/// Metronome beat pattern types
#[derive(Debug, PartialEq, Eq, Clone)]
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

impl From<&BeatPatternType> for char {
    fn from(beat_pattern: &BeatPatternType) -> char {
        match beat_pattern {
            BeatPatternType::Accent => '!',
            BeatPatternType::Beat => '+',
            BeatPatternType::Pause => '.',
        }
    }
}

impl Display for BeatPatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res: char = self.into();
        write!(f, "{}", res)
    }
}

/// Metronome beat pattern
#[derive(Debug, Clone)]
pub struct BeatPattern {
    pub pattern: Vec<BeatPatternType>,
    pub index: Option<usize>,
}

impl BeatPattern {
    pub fn new(pattern: Vec<BeatPatternType>) -> BeatPattern {
        BeatPattern {
            pattern,
            index: None,
        }
    }

    /// String with the current beat marked
    pub fn to_string_with_current_beat(&self) -> String {
        let mut res = String::new();
        for (idx, beat) in self.pattern.iter().enumerate() {
            if Some(idx) == self.index {
                res.extend(
                    format!(
                        "{}{}{}",
                        Attribute::Underlined,
                        beat,
                        Attribute::NoUnderline
                    )
                    .chars(),
                );
            } else {
                res.push(beat.into());
            }
        }
        res
    }
}

impl TryFrom<&str> for BeatPattern {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut result = BeatPattern {
            pattern: Vec::with_capacity(value.len()),
            index: None,
        };
        for element in value.chars() {
            result.pattern.push(BeatPatternType::try_from(&element)?);
        }
        Ok(result)
    }
}

impl Display for BeatPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::new();
        for beat in &self.pattern {
            res.push(beat.into());
        }
        write!(f, "{}", res)
    }
}

pub struct StreamWrapper {
    stream: Stream,
    start_time: Instant,
}

/// A metronome sound player that realizes the beat playback
// #[derive(Debug)]
pub struct BeatPlayer {
    pub bpm: u16,
    pub beat_value: u16,
    pub beat: ToneConfiguration,
    pub ac_beat: ToneConfiguration,
    pub beat_pattern: BeatPattern,
    stream: Option<StreamWrapper>,
    start_stop_mtx: Mutex<()>,
}

impl ReplApp for BeatPlayer {
    fn get_status(&mut self) -> String {
        self.update_pattern_counter();
        format!(
            "pattern: {}  value: 1/{} bpm: {}  !: {:.3}Hz  +:{:.3}Hz",
            &self.beat_pattern.to_string_with_current_beat(),
            &self.beat_value,
            &self.bpm,
            &self.ac_beat.frequency,
            &self.beat.frequency
        )
    }

    fn get_event_interval(&self) -> Duration {
        let events_per_sec = self.bpm as f64 / 60.0;
        std::time::Duration::from_secs_f64(1.0 / events_per_sec)
    }
}

impl Display for BeatPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "bpm: {:4}, beat_value: 1/{}, pattern: {:?}, accent: {:.2}Hz, normal: {:.2}Hz, \
            playing: {}",
            self.bpm,
            self.beat_value,
            self.beat_pattern,
            self.ac_beat.frequency,
            self.beat.frequency,
            self.is_playing()
        )
    }
}

impl BeatPlayer {
    pub fn new(
        bpm: u16,
        beat_value: u16,
        beat: ToneConfiguration,
        ac_beat: ToneConfiguration,
        beat_pattern: BeatPattern,
    ) -> BeatPlayer {
        BeatPlayer {
            bpm,
            beat_value,
            beat,
            ac_beat,
            beat_pattern,
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
        if let Some(x) = self.stream.as_mut() {
            x.stream.pause().expect("Error during pause");
        };
        self.stream = None;
        self.beat_pattern.index = None;
    }

    /// Set the beat pattern
    ///
    /// Stops and resumes playback if playback is running
    pub fn set_pattern(&mut self, beat_pattern: &BeatPattern) -> Result<(), String> {
        if beat_pattern.pattern.is_empty() {
            return Err("Beat pattern is empty, will not change anything".to_string());
        }
        let restart = if self.is_playing() {
            self.stop();
            true
        } else {
            false
        };

        let previous_pattern = beat_pattern.pattern.clone();
        self.beat_pattern.pattern.clone_from(&beat_pattern.pattern);

        if restart && self.play_beat().is_err() {
            self.beat_pattern.pattern = previous_pattern;
            Err("New pattern does not seem to work, returning to previous pattern".to_string())
        } else {
            Ok(())
        }
    }

    /// Set the beat value
    ///
    /// The default value is 4 which means the beat battern is played in a x/4 measure
    /// where x is the number of beats in the beat pattern.
    ///
    /// Stops and resumes playback if playback is running
    pub fn set_beat_value(&mut self, beat_value: u16) -> bool {
        if beat_value == 0 {
            return false;
        }

        let restart = if self.is_playing() {
            self.stop();
            true
        } else {
            false
        };

        let previous_beat_value = self.beat_value;
        self.beat_value = beat_value;

        if restart && self.play_beat().is_err() {
            self.beat_value = previous_beat_value;
            false
        } else {
            true
        }
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

        if restart && self.play_beat().is_err() {
            self.bpm = previous_bpm;
            false
        } else {
            true
        }
    }

    /// Set pitches for accent and normal beat
    ///
    /// Stops and resumes playback if playback is running
    pub fn set_pitches(&mut self, accent_pitch: f64, normal_pitch: f64) -> Result<(), String> {
        let check_pitch_bounds = |x: f64| -> Result<(), String> {
            if (20.0..=20000.0).contains(&x) {
                Ok(())
            } else {
                Err(format!("Value {} out of range", x))
            }
        };
        check_pitch_bounds(accent_pitch)?;
        check_pitch_bounds(normal_pitch)?;

        let restart = if self.is_playing() {
            self.stop();
            true
        } else {
            false
        };

        self.ac_beat.frequency = accent_pitch;
        self.beat.frequency = normal_pitch;

        if restart {
            self.play_beat()?;
        }

        Ok(())
    }

    fn update_pattern_counter(&mut self) {
        if let Some(stream) = &self.stream {
            if self.beat_pattern.index.is_some() {
                let elapsed_seconds = (Instant::now() - stream.start_time).as_secs_f64();
                let beats_per_second = self.bpm as f64 / 60.0;
                let played_beats = (elapsed_seconds * beats_per_second).floor() as usize;
                self.beat_pattern.index = Some(played_beats % self.beat_pattern.pattern.len());
            }
        };
    }

    fn _fill_playback_buffer(
        &self,
        sample_rate: f64,
        channels: usize,
    ) -> Result<AudioSignal<f32>, &'static str> {
        // Create the playback buffer over which the output loops
        // Use self.beat and silence to fill the buffer
        if self.beat.frequency <= 0.0 || self.ac_beat.frequency <= 0.0 {
            return Err("Tone Configuration not applicable");
        }
        let mut beat = AudioSignal::generate_tone(&self.beat);
        let mut ac_beat = AudioSignal::generate_tone(&self.ac_beat);

        // filter tones
        beat.highpass_20hz();
        beat.lowpass_20khz();
        ac_beat.highpass_20hz();
        ac_beat.lowpass_20khz();

        // fade in and out to avoid click and pop noises
        let fade_time = 0.01;
        beat.fade_in_out(fade_time, fade_time).unwrap();
        ac_beat.fade_in_out(fade_time, fade_time).unwrap();

        let beats_per_minute = self.bpm as f64 * self.beat_value as f64 / BASE_BEAT_VALUE as f64;
        let samples_per_beat = ((60.0 * sample_rate) / beats_per_minute).round() as isize;

        let silence_samples = samples_per_beat - beat.signal.len() as isize;
        if silence_samples < 0 {
            return Err("Beat to long to play at current bpm");
        }

        let ac_silence_samples = samples_per_beat - ac_beat.signal.len() as isize;
        if ac_silence_samples < 0 {
            return Err("Accentuated beat to long to play at current bpm");
        }

        // prepare the playback buffer
        let (ac_beat_count, beat_count, pause_count) = {
            let mut a = 0;
            let mut b = 0;
            let mut c = 0;
            for bpt in &self.beat_pattern.pattern {
                match bpt {
                    BeatPatternType::Accent => a += 1,
                    BeatPatternType::Beat => b += 1,
                    BeatPatternType::Pause => c += 1,
                }
            }
            (a, b, c)
        };

        let playback_buffer_samples = ac_beat_count
            * (ac_beat.signal.len() + ac_silence_samples as usize)
            + beat_count * (beat.signal.len() + silence_samples as usize)
            + pause_count * (samples_per_beat as usize);

        let mut playback_buffer = AudioSignal {
            signal: Vec::with_capacity(playback_buffer_samples),
            index: 0,
            tone: ToneConfiguration {
                frequency: 0.0,
                sample_rate,
                length: samples_to_time(playback_buffer_samples, sample_rate),
                overtones: 0,
                channels: 1,
            },
        };

        for beat_type in &self.beat_pattern.pattern {
            match beat_type {
                BeatPatternType::Accent => {
                    playback_buffer
                        .signal
                        .extend_from_slice(&ac_beat.signal[0..]);
                    for _ in 0..ac_silence_samples {
                        playback_buffer.signal.push(0f32);
                    }
                }
                BeatPatternType::Beat => {
                    playback_buffer.signal.extend_from_slice(&beat.signal[0..]);
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

        playback_buffer = {
            if channels > 1 {
                playback_buffer.channels_from_mono(channels).unwrap()
            } else {
                playback_buffer
            }
        };

        Ok(playback_buffer)
    }

    pub fn play_beat(&mut self) -> Result<(), String> {
        let lockguard = self.start_stop_mtx.try_lock();

        if lockguard.is_err() {
            return Err("Cannot start beat playback, it is already running".into());
        }

        let audio_host = cpal::default_host();
        let device = match audio_host.default_output_device() {
            Some(x) => x,
            None => return Err(format!("No audio device for {:?}", audio_host.id())),
        };
        let default_config = {
            match device.default_output_config() {
                Ok(x) => x,
                Err(y) => {
                    return Err(format!(
                        "No output configuration on default output device: {:?}",
                        y
                    ))
                }
            }
        };

        let playback_buffer = self._fill_playback_buffer(
            default_config.sample_rate().0 as f64,
            default_config.channels() as usize,
        )?;

        self.stream = Some(StreamWrapper {
            stream: create_cpal_stream(device, default_config, playback_buffer)?,
            start_time: Instant::now(),
        });
        self.beat_pattern.index = Some(0);

        match self.stream.as_mut().unwrap().stream.play() {
            Ok(_) => (),
            Err(_) => {
                self.stream = None;
                return Err("Something went wrong with beat playback".into());
            }
        };

        // everything was fine fine
        Ok(())
    }
}

fn create_cpal_stream(
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    playback_buffer: AudioSignal<f32>,
) -> Result<Stream, String> {
    let sampletype = config.sample_format();
    let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
    let my_config = config.into();

    //TODO: unify these lambdas somehow
    let stream = match sampletype {
        SampleFormat::F32 => {
            let mut playback_buffer: AudioSignal<f32> = playback_buffer;
            device.build_output_stream(
                &my_config,
                move |data, _| {
                    for sample in data.iter_mut() {
                        *sample = playback_buffer.get_next_sample();
                    }
                },
                err_fn,
                None,
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
                None,
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
                None,
            )
        }
        _ => todo!(),
    };

    match stream {
        Ok(stream) => Ok(stream),
        Err(x) => Err(format!(
            "Streamconfig {:?} is not supported, got error: {:?}",
            my_config, x
        )),
    }
}
