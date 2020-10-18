extern crate alsa;
use alsa::pcm;

use crate::audiosignal::settings;
use crate::audiosignal::AudioSignal;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::vec::Vec;

/// A metronome sound player that realizes the beat playback
#[derive(Debug)]
pub struct BeatPlayer {
    pub bpm: u16,
    pub beat: AudioSignal,
    pub accentuated_beat: AudioSignal,
    pub pattern: Vec<bool>,

    stop_request: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
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
        accentuated_beat: AudioSignal,
        pattern: Vec<bool>,
    ) -> BeatPlayer {
        BeatPlayer {
            bpm,
            beat,
            accentuated_beat,
            pattern,
            stop_request: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.thread.is_some()
    }

    pub fn stop(&mut self) {
        match self.thread {
            Some(_) => {
                let mut join_handle = None;
                std::mem::swap(&mut join_handle, &mut self.thread);
                self.stop_request.store(true, Ordering::SeqCst);
                join_handle.unwrap().join().unwrap();
                self.thread = None;
            }
            None => (),
        }
    }

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

    pub fn play_beat(&mut self) -> Result<(), alsa::Error> {
        println!(
            "Starting playback with {} bpm with pattern {:?}",
            self.bpm, self.pattern
        );
        // Create the playback buffer over which the output loops
        // Use self.beat and silence to fill the buffer
        if self.beat.signal.is_empty() {
            return Err(alsa::Error::unsupported("No beat to play"));
        }

        if self.thread.is_some() {
            return Err(alsa::Error::unsupported("Playback is already running"));
        }

        let samples_per_beat = ((60.0 * settings::SAMPLERATE) / self.bpm as f64).round() as isize;

        let silence_samples = samples_per_beat as isize - self.beat.signal.len() as isize;
        if silence_samples < 0 {
            return Err(alsa::Error::unsupported("Beat to long to play"));
        }

        // prepare the playback buffer
        let mut playback_buffer = AudioSignal {
            signal: self.beat.signal.to_vec(),
        };

        for _ in 0..silence_samples {
            playback_buffer.signal.push(0);
        }

        let stop_request = Arc::clone(&self.stop_request);
        self.thread = Some(thread::spawn(move || {
            let pcm_handle = init_audio().unwrap();
            let io = pcm_handle.io_i16().unwrap();

            if pcm_handle.state() != pcm::State::Running {
                pcm_handle.start().unwrap();
            };
            while !stop_request.load(Ordering::SeqCst) {
                io.writei(&playback_buffer.signal[..]).unwrap();
            }
            stop_request.store(false, Ordering::SeqCst);

            pcm_handle.drain().unwrap();
        }));

        Ok(())
    }
}

fn init_audio() -> Result<alsa::pcm::PCM, alsa::Error> {
    let pcm_handle = pcm::PCM::new("default", alsa::Direction::Playback, false)?;
    {
        let pcm_hw_params = pcm::HwParams::any(&pcm_handle)?;
        pcm_hw_params.set_format(pcm::Format::s16())?;
        pcm_hw_params.set_access(pcm::Access::RWInterleaved)?;
        pcm_hw_params.set_channels(1)?;
        pcm_hw_params.set_rate(settings::SAMPLERATE.round() as u32, alsa::ValueOr::Nearest)?;
        pcm_hw_params.set_rate_resample(true)?;
        let period_size = (settings::SAMPLERATE * settings::ALSA_MIN_WRITE).round() as i64;
        pcm_hw_params.set_period_size_near(period_size, alsa::ValueOr::Nearest)?;
        pcm_hw_params.set_buffer_size_near(2 * period_size)?;
        pcm_handle.hw_params(&pcm_hw_params)?;
    }
    Ok(pcm_handle)
}
