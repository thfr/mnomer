extern crate alsa;

use alsa::pcm;

use mnomer;
use mnomer::audiosignal::AudioSignal;
use mnomer::audiosignal::settings;


struct BeatPlayer {
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

struct Repl {
    commands: Vec<(String, fn(&str))>,
}

impl Repl {}

fn main() {
    let freq = 500.0;
    let length = 0.002;
    let mut sine = AudioSignal::generate_sine(freq, length, 3);
    println!("Sine {}Hz, {}s:", freq, length);
    for sample in &sine.signal {
        print!("{} ", sample);
    }
    println!();

    println!("Highpass filter with 20Hz and lowpass filter with 20kHz");
    sine.highpass_20hz();
    sine.lowpass_20khz();
    for sample in &sine.signal {
        print!("{} ", sample);
    }
    println!();

    let fade_time = 0.00025;
    match sine.fade_in_out(fade_time, fade_time) {
        Ok(_) => {
            println!("Fade in {}s, fade out {}s:", fade_time, fade_time);
            for sample in &sine.signal {
                print!("{} ", sample);
            }
        }
        _ => {
            println!("fade_in_out() did not work");
        }
    }
    println!();
}
