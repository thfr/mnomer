extern crate alsa;

use std::f64;
use std::i16;

// use alsa::*;
pub mod settings {
    pub const samplerate: u32 = 48000;
    pub const alsa_min_write: f64 = 0.1; // [s]
    pub const fade_min_time: f64 = 0.01; // [s]
    pub const fade_min_percentage: u32 = 48000;
    pub const sine_max_amplitude: f64 = 0.75;
}

fn generate_sine(freq: u32, length: f64) -> Vec<i16> {
    let num_samples = (length * settings::samplerate as f64).round() as usize;
    let mut sine: Vec<i16> = Vec::with_capacity(num_samples);
    for sam in 0..num_samples {
        let sine_value =
            (sam as f64 * f64::consts::PI * (freq as f64) / (settings::samplerate as f64)).sin();
        sine.push((sine_value * settings::sine_max_amplitude * i16::MAX as f64) as i16);
    }
    sine
}

struct BeatPlayer {
    beat: Vec<i16>,
    accentuated_beat: Vec<i16>,
    pattern: Vec<bool>,
}

struct Repl {
    commands: Vec<(String, fn(&str))>,
}

fn main() {
    let sine = generate_sine(500, 0.05);
    for sample in sine {
        print!("{} ", sample);
    }
    println!();
}
