extern crate alsa;
use alsa::pcm;

use crate::audiosignal::AudioSignal;
use crate::audiosignal::settings;

pub struct BeatPlayer {
    bpm: u16,
    beat: AudioSignal,
    accentuated_beat: AudioSignal,
    playback_buffer: AudioSignal,
    pattern: Vec<bool>,
}

impl BeatPlayer {
    pub fn play_beat(&mut self) -> Result<(), (alsa::Error)> {
        let pcm_handle = match pcm::PCM::new("default", alsa::Direction::Playback, false) {
            Ok(pcm) => pcm,
            Err(e) => return Err(e),
        };

        let pcm_hw_params = pcm::HwParams::any(&pcm_handle)?;
        pcm_hw_params.set_format(pcm::Format::s16())?;
        pcm_hw_params.set_access(pcm::Access::RWInterleaved)?;
        pcm_hw_params.set_rate(settings::SAMPLERATE.round() as u32, alsa::ValueOr::Nearest)?;
        pcm_hw_params.set_rate_resample(true)?;
        pcm_handle.hw_params(&pcm_hw_params)?;

        Ok(())
    }
}

