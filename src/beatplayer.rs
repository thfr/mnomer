extern crate alsa;
use alsa::pcm;

use crate::audiosignal::{settings, time_in_samples, AudioSignal};

use std::cmp::min;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::vec::Vec;

/// A metronome sound player that realizes the beat playback
#[derive(Debug)]
pub struct BeatPlayer {
    pub bpm: u16,
    pub beat: AudioSignal,
    pub ac_beat: AudioSignal,
    pub pattern: Vec<bool>,

    stop_request: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    starting: Mutex<()>,
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
        pattern: Vec<bool>,
    ) -> BeatPlayer {
        BeatPlayer {
            bpm,
            beat,
            ac_beat,
            pattern,
            stop_request: Arc::new(AtomicBool::new(false)),
            thread: None,
            starting: Mutex::new(()),
        }
    }

    /// Check whether the beat playback is running or starting
    pub fn is_playing(&self) -> bool {
        let lockguard = self.starting.try_lock();
        if lockguard.is_err() || self.thread.is_some() {
            true
        } else {
            false
        }
    }

    /// Stop the beat playback
    pub fn stop(&mut self) {
        match self.thread {
            Some(_) => {
                println!("Stopping playback");
                let mut join_handle = None;
                std::mem::swap(&mut join_handle, &mut self.thread);
                self.stop_request.store(true, Ordering::SeqCst);
                join_handle.unwrap().join().unwrap();
                self.thread = None;
            }
            None => (),
        }
    }

    /// Set the beats per minute
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

    /// Start the beat playback
    pub fn play_beat(&mut self) -> Result<(), alsa::Error> {
        // acquire playback lock
        let lockguard = self.starting.try_lock();
        if lockguard.is_err() || self.thread.is_some() {
            return Err(alsa::Error::unsupported(
                "Cannot start beat playback, it is already running",
            ));
        }

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
            return Err(alsa::Error::unsupported(
                "Beat to long to play at current bpm",
            ));
        }

        let ac_silence_samples =
            samples_per_beat as isize - self.ac_beat.signal.len() as isize;
        if ac_silence_samples < 0 {
            return Err(alsa::Error::unsupported(
                "Accentuated beat to long to play at current bpm",
            ));
        }

        // prepare the playback buffer
        let ac_beat_count = self.pattern.iter().filter(|&x| *x == true).count();
        let beat_count = self.pattern.iter().filter(|&x| *x == false).count();
        let playback_buffer_samples = ac_beat_count
            * (self.ac_beat.signal.len() + ac_silence_samples as usize)
            + beat_count * (self.beat.signal.len() + silence_samples as usize);
        let mut playback_buffer = AudioSignal {
            signal: Vec::with_capacity(playback_buffer_samples),
        };
        for is_accentuated_beat in &self.pattern {
            if *is_accentuated_beat {
                playback_buffer
                    .signal
                    .extend_from_slice(&self.ac_beat.signal[0..]);
                for _ in 0..ac_silence_samples {
                    playback_buffer.signal.push(0);
                }
            } else {
                playback_buffer
                    .signal
                    .extend_from_slice(&self.beat.signal[0..]);
                for _ in 0..silence_samples {
                    playback_buffer.signal.push(0);
                }
            }
        }

        let stop_request = Arc::clone(&self.stop_request);
        self.thread = Some(thread::spawn(move || {
            let pcm_handle = init_audio().unwrap();
            let io = pcm_handle.io_i16().unwrap();

            if pcm_handle.state() != pcm::State::Running {
                pcm_handle.start().unwrap();
            };

            // make the write operations to the ALSA device independent from the size of the
            // playback buffer by only giving fixed size slices to `io.writei`
            let samples_per_write_op = time_in_samples(0.1);
            let buffer_splits =
                (playback_buffer.signal.len() as f64 / samples_per_write_op as f64).ceil() as usize;
            while !stop_request.load(Ordering::SeqCst) {
                for split_index in 0..buffer_splits {
                    let start_index = split_index * samples_per_write_op;
                    let end_index = min(
                        start_index + samples_per_write_op,
                        playback_buffer.signal.len(),
                    );
                    if !stop_request.load(Ordering::SeqCst) {
                        io.writei(&playback_buffer.signal[start_index..end_index])
                            .unwrap();
                    } else {
                        break;
                    }
                }
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
