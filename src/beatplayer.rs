extern crate alsa;
use alsa::pcm;

use crate::audiosignal::settings;
use crate::audiosignal::AudioSignal;

pub struct BeatPlayer {
    pub bpm: u16,
    pub beat: AudioSignal,
    pub accentuated_beat: AudioSignal,
    pub playback_buffer: AudioSignal,
    pub pattern: Vec<bool>,
}

impl BeatPlayer {
    pub fn play_beat(self) -> Result<(), (alsa::Error)> {
        let pcm_handle = pcm::PCM::new("default", alsa::Direction::Playback, false)?;
        let pcm_hw_params = pcm::HwParams::any(&pcm_handle)?;
        pcm_hw_params.set_format(pcm::Format::s16())?;
        pcm_hw_params.set_access(pcm::Access::RWInterleaved)?;
        pcm_hw_params.set_rate(settings::SAMPLERATE.round() as u32, alsa::ValueOr::Nearest)?;
        pcm_hw_params.set_rate_resample(true)?;
        pcm_handle.hw_params(&pcm_hw_params)?;
        let io = pcm_handle.io_i16()?;

        for _ in 0..100 {
            io.writei(&self.beat.signal[..])?;
        }

        if pcm_handle.state() != pcm::State::Running {
            pcm_handle.start()?;
        };

        pcm_handle.drain()?;

        Ok(())
    }
}
