use std::f64;
use std::i16;

use std::ops::*;

pub type AudioSample = i16;

pub mod settings {
    pub const SAMPLERATE: f64 = 48000.0;
    pub const ALSA_MIN_WRITE: f64 = 0.1; // [s]
    pub const FADE_MIN_TIME: f64 = 0.01; // [s]
    pub const FADE_MIN_PERCENTAGE: f64 = 0.3;
    pub const SINE_MAX_AMPLITUDE: f64 = 0.75;
}

pub fn time_in_samples(time: f64) -> usize {
    (time * settings::SAMPLERATE).round() as usize
}

pub fn samples_to_time(samples: usize) -> f64 {
    samples as f64 / settings::SAMPLERATE
}

pub struct AudioSignal {
    pub signal: Vec<AudioSample>,
}

impl Add for AudioSignal {
    type Output = AudioSignal;

    fn add(self, other: AudioSignal) -> AudioSignal {
        let (smaller, larger) = if self.signal.len() < other.signal.len() {
            (&self.signal, &other.signal)
        } else {
            (&other.signal, &self.signal)
        };
        AudioSignal {
            signal: {
                let mut s = larger.to_vec();
                for idx in 0..smaller.len() {
                    s[idx] += smaller[idx];
                }
                s
            },
        }
    }
}

impl AddAssign for AudioSignal {
    fn add_assign(&mut self, other: AudioSignal) {
        if self.signal.len() < other.signal.len() {
            self.signal.resize(other.signal.len(), 0);
        }
        for idx in 0..other.signal.len() {
            self.signal[idx] += other.signal[idx];
        }
    }
}

impl AudioSignal {
    pub fn generate_sine(freq: f64, length: f64, overtones: u8) -> AudioSignal {
        let num_samples = (length * settings::SAMPLERATE).round() as usize;
        let mut sine: Vec<AudioSample> = Vec::with_capacity(num_samples);
        for sam in 0..num_samples {
            let x = sam as f64;
            let pi = f64::consts::PI;
            let srate = settings::SAMPLERATE;
            let mut value = (x * 2.0 * pi * freq / srate).sin();
            let ot_mul_gain = 0.5;
            let mut ot_gain = 0.5;
            for ot in 0..overtones {
                let tone_freq = (ot + 2) as f64 * freq;
                value += ot_gain * (x * 2.0 * pi * tone_freq / srate).sin();
                ot_gain *= ot_mul_gain;
            }
            sine.push(
                (value * settings::SINE_MAX_AMPLITUDE * AudioSample::max_value() as f64).round()
                    as AudioSample,
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
