extern crate alsa;
use alsa::pcm;

use crate::audiosignal::settings;
use crate::audiosignal::AudioSignal;

use std::sync::Arc;
use std::thread;

pub struct BeatPlayer {
    pub bpm: u16,
    pub beat: AudioSignal,
    pub accentuated_beat: AudioSignal,
    pub pattern: Vec<bool>,
    stop_request: std::sync::Arc<bool>,
    thread: std::thread::JoinHandle<dyn Fn() + 'static>,
}

impl BeatPlayer {
    pub fn stop(&mut self) {
        *self.stop_request = true;
    }

    pub fn play_beat(&mut self) -> Result<(), alsa::Error> {
        println!("Starting playback with {} bpm", self.bpm);
        // Create the playback buffer over which the output loops
        // Use self.beat and silence to fill the buffer
        if self.beat.signal.len() == 0 {
            return Err(alsa::Error::unsupported("No beat to play"));
        }

        let samples_per_beat = ((60.0 * settings::SAMPLERATE) / self.bpm as f64).round() as isize;

        let silence_samples = samples_per_beat as isize - self.beat.signal.len() as isize;
        if silence_samples < 0 {
            return Err(alsa::Error::unsupported("Beat to long to play"));
        }

        // prepare the playback buffer
        let mut playback_buffer: AudioSignal;
        playback_buffer.signal = self.beat.signal.to_vec();

        for _ in 0..silence_samples {
            playback_buffer.signal.push(0);
        }

        let pcm_handle = self.init_audio()?;
        let io = pcm_handle.io_i16()?;

        if pcm_handle.state() != pcm::State::Running {
            pcm_handle.start()?;
        };


        self.thread = thread::spawn(move || {
            while !*self.stop_request {
                io.writei(&playback_buffer.signal[..]).unwrap();
            }

            pcm_handle.drain().unwrap();
        });

        Ok(())
    }

    fn init_audio(&self) -> Result<alsa::pcm::PCM, alsa::Error> {
        let pcm_handle = pcm::PCM::new("default", alsa::Direction::Playback, false)?;
        {
            let pcm_hw_params = pcm::HwParams::any(&pcm_handle)?;
            pcm_hw_params.set_format(pcm::Format::s16())?;
            pcm_hw_params.set_access(pcm::Access::RWInterleaved)?;
            pcm_hw_params.set_rate(settings::SAMPLERATE.round() as u32, alsa::ValueOr::Nearest)?;
            pcm_hw_params.set_rate_resample(true)?;
            pcm_handle.hw_params(&pcm_hw_params)?;
        }
        Ok(pcm_handle)
    }
}
