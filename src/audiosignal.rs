use std::f64;
use std::i16;

use std::ops::{Add, AddAssign, Mul, MulAssign};

pub mod settings {
    pub const SINE_MAX_AMPLITUDE: f64 = 0.75;
}

pub fn time_in_samples(time: f64, sample_rate: f64) -> usize {
    (time * sample_rate).round() as usize
}

#[allow(dead_code)]
/// Convert number of samples to seconds
pub fn samples_to_time(samples: usize, sample_rate: f64) -> f64 {
    samples as f64 / sample_rate
}

pub fn freqency_relative_semitone_equal_temperament(base: f64, semitone: f64) -> f64 {
    base * 2f64.powf(semitone / 12f64)
}

#[derive(Debug, Clone)]
pub struct ToneConfiguration {
    pub sample_rate: f64,
    pub frequency: f64,
    pub overtones: u8,
    pub length: f64,
    pub channels: usize,
}

#[derive(Debug, Clone)]
pub struct AudioSignal<T> {
    pub signal: Vec<T>,
    pub index: usize,
    pub tone: ToneConfiguration,
}

impl<T: Copy> AudioSignal<T> {
    pub fn get_next_sample(&mut self) -> T {
        self.index = (self.index + 1) % self.signal.len();
        self.signal[self.index as usize]
    }
}

impl From<AudioSignal<f32>> for AudioSignal<u16> {
    fn from(audio_signal: AudioSignal<f32>) -> Self {
        let mut audio: Vec<u16> = Vec::with_capacity(audio_signal.signal.len());
        for sample in audio_signal.signal.into_iter() {
            let saturated_sample = if sample > 1f32 {
                1f32
            } else if sample < -1f32 {
                -1f32
            } else {
                sample
            };
            audio.push((saturated_sample * (u16::MAX / 2) as f32).round() as u16);
        }
        AudioSignal {
            signal: audio,
            index: 0,
            tone: audio_signal.tone,
        }
    }
}

impl From<AudioSignal<f32>> for AudioSignal<i16> {
    fn from(audio_signal: AudioSignal<f32>) -> Self {
        let mut audio: Vec<i16> = Vec::with_capacity(audio_signal.signal.len());
        for sample in audio_signal.signal.into_iter() {
            let saturated_sample = if sample > 1f32 {
                1f32
            } else if sample < -1f32 {
                -1f32
            } else {
                sample
            };
            audio.push((saturated_sample * i16::MAX as f32).round() as i16);
        }
        AudioSignal {
            signal: audio,
            index: 0,
            tone: audio_signal.tone,
        }
    }
}

impl Add for AudioSignal<f32> {
    type Output = AudioSignal<f32>;

    fn add(self, other: AudioSignal<f32>) -> AudioSignal<f32> {
        let mut new_as = AudioSignal {
            signal: self.signal.to_vec(),
            index: 0,
            tone: self.tone,
        };
        new_as += other;
        new_as
    }
}

impl AddAssign for AudioSignal<f32> {
    fn add_assign(&mut self, other: AudioSignal<f32>) {
        if self.signal.len() < other.signal.len() {
            self.signal.resize(other.signal.len(), self.signal[0]);
        }
        for idx in 0..other.signal.len() {
            self.signal[idx] += other.signal[idx];
        }
    }
}

impl Mul<f64> for AudioSignal<f32> {
    type Output = AudioSignal<f32>;

    fn mul(self, factor: f64) -> AudioSignal<f32> {
        let mut new_as = AudioSignal {
            signal: self.signal.to_vec(),
            index: 0,
            tone: self.tone,
        };
        new_as *= factor;
        new_as
    }
}

impl MulAssign<f64> for AudioSignal<f32> {
    fn mul_assign(&mut self, factor: f64) {
        for sample in self.signal.iter_mut() {
            *sample = ((*sample) as f64 * factor) as f32;
        }
    }
}

impl AudioSignal<f32> {
    pub fn generate_tone(tone: &ToneConfiguration) -> AudioSignal<f32> {
        // base signal
        let mut signal = AudioSignal::generate_sine(tone.frequency, tone.length, tone.sample_rate);

        // add overtones
        for freq_factor in 2..(tone.overtones + 2) {
            signal += AudioSignal::generate_sine(
                freq_factor as f64 * tone.frequency,
                tone.length,
                tone.sample_rate,
            ) * 0.5;
        }

        signal
    }

    fn generate_sine(freq: f64, length: f64, sample_rate: f64) -> AudioSignal<f32> {
        let tone = ToneConfiguration {
            frequency: freq,
            length,
            sample_rate,
            overtones: 0,
            channels: 1,
        };
        let pi = f64::consts::PI;
        let amplitude = settings::SINE_MAX_AMPLITUDE as f64;

        let num_samples = (length * sample_rate).round() as usize;
        let mut audio_signal = AudioSignal {
            signal: Vec::with_capacity(num_samples),
            index: 0,
            tone,
        };

        for sam in 0..num_samples {
            let x = sam as f64;
            let value = amplitude * (x * 2.0 * pi * freq / sample_rate).sin();
            audio_signal.signal.push(value as f32);
        }
        audio_signal
    }

    pub fn channels_from_mono(&self, channels: usize) -> Result<AudioSignal<f32>, String> {
        if self.tone.channels != 1 {
            return Err("Can only use mono AudioSignals".into());
        }
        let mut audio_signal = AudioSignal {
            signal: Vec::with_capacity(channels * self.signal.len()),
            index: 0,
            tone: self.tone.clone(),
        };
        audio_signal.tone.channels = channels;

        for &sample in self.signal.iter() {
            for _ in 0..channels {
                audio_signal.signal.push(sample);
            }
        }

        Ok(audio_signal)
    }

    pub fn fade_in_out(&mut self, fade_in_time: f64, fade_out_time: f64) -> Result<(), ()> {
        // early return
        if fade_in_time < 0.0 || fade_out_time < 0.0 {
            return Err(());
        }
        // *Exponential Fading* is used because it is more pleasant to ear than linear fading.
        //
        // A factor with changing value is multiplied to each sample of the fading period.
        // The factor must be increased by multiplying it with a constant ratio until it reaches
        // 1.0.  Therefore the factor must have a starting value > 0.0.
        //    fs * (r ** steps) = 1         (discrete form: f[n+1] = f[n] * r , while f[n+1] <= 1)
        //    r ** steps  = 1 / fs
        //    r = (1 / fs) ** (1 / steps)
        //       where fs = factor at start
        //              r = ratio
        let start_value = 1.0 / i16::MAX as f64;
        let fade_in_samples =
            time_in_samples(fade_in_time, self.tone.sample_rate).min(self.signal.len());
        let fade_out_samples =
            time_in_samples(fade_out_time, self.tone.sample_rate).min(self.signal.len());
        let fade_in_ratio = (1.0 / start_value).powf(1.0 / fade_in_samples as f64);
        let fade_out_ratio = (1.0 / start_value).powf(-1.0 / fade_out_samples as f64);

        // fade in
        let mut fade_in_factor = start_value;
        for index in 0..fade_in_samples {
            let sample = self.signal[index] as f64;
            self.signal[index] = (sample * fade_in_factor) as f32;
            fade_in_factor *= fade_in_ratio;
        }

        // fade out
        let mut fade_out_factor = 1.0;
        for index in (self.signal.len() - fade_out_samples)..self.signal.len() {
            let sample = self.signal[index] as f64;
            self.signal[index] = (sample * fade_out_factor) as f32;
            fade_out_factor *= fade_out_ratio;
        }

        Ok(())
    }

    pub fn highpass_20hz(&mut self) {
        /* Digital filter designed by mkfilter/mkshape/gencode   A.J. Fisher
         *    Command line: /www/usr/fisher/helpers/mkfilter -Bu -Hp -o 2 -a 4.1666666667e-04
         *    0.0000000000e+00 -l */

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
            *sample = yv[2] as f32;
        }
    }

    pub fn lowpass_20khz(&mut self) {
        /* Digital filter designed by mkfilter/mkshape/gencode   A.J. Fisher
         *    Command line: /www/usr/fisher/helpers/mkfilter -Bu -Lp -o 2 -a 4.1666666667e-01
         *    0.0000000000e+00 -l */

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
            *sample = yv[2] as f32;
        }
    }
}
