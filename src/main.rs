extern crate alsa;

use std::f64;
use std::i16;

type AudioSample = i16;

// use alsa::*;
pub mod settings {
    pub const SAMPLERATE: f64 = 48000.0;
    pub const ALSA_MIN_WRITE: f64 = 0.1; // [s]
    pub const FADE_MIN_TIME: f64 = 0.01; // [s]
    pub const FADE_MIN_PERCENTAGE: f64 = 0.3;
    pub const SINE_MAX_AMPLITUDE: f64 = 0.75;
}

fn time_in_samples(time: f64) -> usize {
    (time * settings::SAMPLERATE).round() as usize
}

fn samples_to_time(samples: usize) -> f64 {
    samples as f64 / settings::SAMPLERATE
}

struct AudioSignal {
    signal: Vec<AudioSample>,
}

impl AudioSignal {
    pub fn generate_sine(freq: f64, length: f64) -> AudioSignal {
        let num_samples = (length * settings::SAMPLERATE).round() as usize;
        let mut sine: Vec<AudioSample> = Vec::with_capacity(num_samples);
        for sam in 0..num_samples {
            let x = sam as f64;
            let pi = f64::consts::PI;
            let srate = settings::SAMPLERATE;
            let value = (x * 2.0 * pi * freq / srate).sin();
            sine.push(
                (value * settings::SINE_MAX_AMPLITUDE * i16::MAX as f64).round() as AudioSample,
            );
        }
        AudioSignal { signal: sine }
    }

    pub fn fade_in_out(&mut self, fade_in_time: f64, fade_out_time: f64) -> Result<(), ()> {
        // early return
        if fade_in_time < 0.0 || fade_in_time < 0.0 {
            return Err(());
        }
        // *Exponential Fading* is used because it is more pleasant to ear than linear fading.
        //
        // A factor with changing value is multiplied to each sample of the fading period.
        // The factor must be increased by multiplying it with a constant ratio that. Therefore the
        // factor must have a starting value > 0.0 .
        //    fs * (r ** steps) = 1         (discrete form: f[n+1] = f[n] * r , while f[n+1] <= 1)
        //    r ** steps  = 1 / fs
        //    r = (1 / fs) ** (1 / steps)
        //       where fs = factor at start
        //              r = ratio
        let start_value = 1.0 / i16::MAX as f64;
        let fade_in_samples = time_in_samples(fade_in_time).min(self.signal.len());
        let fade_out_samples = time_in_samples(fade_out_time).min(self.signal.len());
        let fade_in_ratio = (1.0 / start_value).powf(1.0 / fade_in_samples as f64);
        let fade_out_ratio = (1.0 / start_value).powf(-1.0 / fade_out_samples as f64);

        // fade in
        let mut fade_in_factor = start_value;
        for index in 0..fade_in_samples {
            let sample = self.signal[index] as f64;
            self.signal[index] = (sample * fade_in_factor).round() as AudioSample;
            fade_in_factor *= fade_in_ratio;
        }

        // fade out
        let mut fade_out_factor = 1.0;
        for index in (self.signal.len() - fade_out_samples)..self.signal.len() {
            let sample = self.signal[index] as f64;
            self.signal[index] = (sample * fade_out_factor).round() as AudioSample;
            fade_out_factor *= fade_out_ratio;
        }

        Ok(())
    }

    pub fn highpass_20hz(&mut self) {
        /* Digital filter designed by mkfilter/mkshape/gencode   A.J. Fisher
         *    Command line: /www/usr/fisher/helpers/mkfilter -Bu -Lp -o 2 -a 4.1666666667e-01
         * 0.0000000000e+00 -l */

        let gain = 1.001852916e+00;

        let mut xv = [0.0, 0.0, 0.0];
        let mut yv = [0.0, 0.0, 0.0];

        for sample in &mut self.signal {
            xv[0] = xv[1];
            xv[1] = xv[2];
            xv[2] = *sample as f64 / gain;
            yv[0] = yv[1];
            yv[1] = yv[2];
            yv[2] =
                (xv[0] + xv[2]) - 2.0 * xv[1] + (-0.9963044430 * yv[0]) + (1.9962976018 * yv[1]);
            *sample = yv[2].round() as AudioSample;
        }
    }
    pub fn lowpass_20khz(&mut self) {
        /* Digital filter designed by mkfilter/mkshape/gencode   A.J. Fisher
         *    Command line: /www/usr/fisher/helpers/mkfilter -Bu -Lp -o 2 -a 4.1666666667e-01
         * 0.0000000000e+00 -l */

        let gain = 1.450734152e+00;

        let mut xv = [0.0, 0.0, 0.0];
        let mut yv = [0.0, 0.0, 0.0];

        for sample in &mut self.signal {
            xv[0] = xv[1];
            xv[1] = xv[2];
            xv[2] = *sample as f64 / gain;
            yv[0] = yv[1];
            yv[1] = yv[2];
            yv[2] =
                (xv[0] + xv[2]) + 2.0 * xv[1] + (-0.4775922501 * yv[0]) + (-1.2796324250 * yv[1]);
            *sample = yv[2].round() as AudioSample;
        }
    }
}

struct BeatPlayer {
    beat: Vec<AudioSample>,
    accentuated_beat: Vec<AudioSample>,
    pattern: Vec<bool>,
}

struct Repl {
    commands: Vec<(String, fn(&str))>,
}

fn main() {
    let freq = 500.0;
    let length = 0.002;
    let mut sine = AudioSignal::generate_sine(freq, length);
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
        },
        _ => {
            println!("fade_in_out() did not work");
        },
    }
    println!();
}
