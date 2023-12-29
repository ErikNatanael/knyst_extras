//! Galactic reverb
//!
//! ported from airwindows Galactic plugin
//! License: MIT
// Original code: Copyright (c) 2016 airwindows, Airwindows uses the MIT license
// Ported code: Copyright 2023 Erik Natanael Gustafsson

use knyst::gen::delay::StaticSampleDelay;
use knyst::gen::GenState;
use knyst::prelude::impl_gen;
use knyst::{Sample, SampleRate};

pub struct Galactic {
    delays_left: [StaticSampleDelay; 12],
    delays_right: [StaticSampleDelay; 12],
    feedback: [[Sample; 4]; 2],
    detune_delay_left: StaticSampleDelay,
    detune_delay_right: StaticSampleDelay,
    lowpass_pre: [Sample; 2],
    lowpass_post: [Sample; 2],
    fpd_l: u32,
    fpd_r: u32,
    oldfpd: f64,
    vib_m: f64,
    iir_al: Sample,
    iir_ar: Sample,
    iir_bl: Sample,
    iir_br: Sample,
}

const GALACTIC_DELAY_TIMES: [usize; 12] = [
    6480, 3660, 1720, 680, 9700, 6000, 2320, 940, 15220, 8460, 4540, 3200,
];

#[impl_gen]
impl Galactic {
    pub fn new() -> Self {
        let mut rng = fastrand::Rng::with_seed(knyst::gen::random::next_randomness_seed());
        Self {
            delays_left: std::array::from_fn(|_| StaticSampleDelay::new(1)),
            delays_right: std::array::from_fn(|_| StaticSampleDelay::new(1)),
            detune_delay_left: StaticSampleDelay::new(1),
            detune_delay_right: StaticSampleDelay::new(1),
            lowpass_pre: [0., 0.],
            lowpass_post: [0., 0.],
            fpd_l: rng.u32(16386..std::u32::MAX),
            fpd_r: rng.u32(16386..std::u32::MAX),
            vib_m: 3.,
            feedback: [[0.0; 4]; 2],
            oldfpd: 429496.7295,
            iir_al: 0.,
            iir_ar: 0.,
            iir_bl: 0.,
            iir_br: 0.,
        }
    }
    pub fn init(&mut self, sample_rate: SampleRate) {
        for (delay, time) in self.delays_left.iter_mut().zip(GALACTIC_DELAY_TIMES) {
            let time = (time as Sample / 44100.) * *sample_rate;
            *delay = StaticSampleDelay::new(time as usize);
        }
        for (delay, time) in self.delays_right.iter_mut().zip(GALACTIC_DELAY_TIMES) {
            let time = (time as Sample / 44100.) * *sample_rate;
            *delay = StaticSampleDelay::new(time as usize);
        }
        // self.detune_delay_left =
        //     StaticSampleDelay::new((0.07054421768707483 * *sample_rate) as usize);
        // self.detune_delay_right =
        //     StaticSampleDelay::new((0.07054421768707483 * *sample_rate) as usize);
        self.detune_delay_left = StaticSampleDelay::new(256);
        self.detune_delay_right = StaticSampleDelay::new(256);
        self.lowpass_pre = [0., 0.];
        self.lowpass_post = [0., 0.];
    }
    pub fn process(
        &mut self,
        left: &[Sample],
        right: &[Sample],
        size: &[Sample],
        replace: &[Sample],
        brightness: &[Sample],
        detune: &[Sample],
        mix: &[Sample],
        left_out: &mut [Sample],
        right_out: &mut [Sample],
        sample_rate: SampleRate,
    ) -> GenState {
        let mut overallscale = 1.0;
        overallscale /= 44100.0;
        overallscale *= *sample_rate;

        // double regen = 0.0625+((1.0-A)*0.0625); // High (0.125) if Replace is low
        // double attenuate = (1.0 - (regen / 0.125))*1.333; // 1.33 if regen is low / replace is high

        let regen = 0.0625 + ((1.0 - replace[0]) * 0.0625);
        let attenuate = (1.0 - (regen / 0.125)) * 1.333; // 1.33 if regen is high / replace is low
        let lowpass = (1.00001 - (1.0 - brightness[0])).powi(2) / (overallscale).sqrt(); // (0.00001 + Brightness).powi(2)/overallscale.sqrt()
        let drift = detune[0].powi(3) * 0.001; // Detune.powi(3) * 0.001
        let size = (size[0] * 0.9) + 0.1;
        let wet = 1.0 - (1.0 - mix[0]).powi(3);

        for (delay_left, delay_right) in self
            .delays_left
            .iter_mut()
            .zip(self.delays_right.iter_mut())
        {
            delay_left.set_delay_length_fraction(size);
            delay_right.set_delay_length_fraction(size);
        }

        // let lengths = [3407., 1823., 859., 331., 4801., 2909., 1153., 461., 7607., 4217., 2269., 1597.];
        // for ((left, right), len) in self.delays_left.iter_mut().zip(self.delays_right.iter_mut()).zip(lengths) {
        //     let len = (len * size) as usize;
        //     left.set_delay_length(len);
        //     right.set_delay_length(len);
        // }

        for (((&input_sample_l, &input_sample_r), output_l), output_r) in left
            .iter()
            .zip(right.iter())
            .zip(left_out.iter_mut())
            .zip(right_out.iter_mut())
        {
            // # Per sample:
            // - If the input is very faint, use the fpd values instead (floating point dither, similar to the last output sample)

            // Apply dither
            let input_sample_l = if input_sample_l.abs() < 1.18e-23 {
                (self.fpd_l as f64 * 1.18e-17) as Sample
            } else {
                input_sample_l
            };
            let input_sample_r = if input_sample_r.abs() < 1.18e-23 {
                (self.fpd_r as f64 * 1.18e-17) as Sample
            } else {
                input_sample_r
            };
            let dry_sample_l = input_sample_l;
            let dry_sample_r = input_sample_r;

            // - vibM cycles 0. - TAU, speed depending on drift (Detune) and the fpdL value last time it reset
            // vibM is phase 0-TAU, speed dpends on drift and fpd
            self.vib_m += self.oldfpd * drift as f64;
            if self.vib_m > (3.141592653589793238 * 2.0) {
                self.vib_m = 0.0;
                self.oldfpd = 0.4294967295 + (self.fpd_l as f64 * 0.0000000000618);
            }

            // - set the fixed size delay (256 frames) to the inputSample at the current position
            self.detune_delay_left
                .write_and_advance(input_sample_l * attenuate);
            self.detune_delay_right
                .write_and_advance(input_sample_r * attenuate);
            // - Get a sample from the aM buffer (lin interp)
            let vib_m_sin = self.vib_m.sin(); // TODO: replace by something faster
            let offset_ml = ((vib_m_sin) + 1.0) * 127.; // 0-256
            let offset_mr = ((self.vib_m + (3.141592653589793238 / 2.0)).sin() + 1.0) * 127.; // 0-256 90 degrees phase shifted
            let working_ml = self.detune_delay_left.position as f64 + offset_ml;
            let working_mr = self.detune_delay_right.position as f64 + offset_mr;
            let input_sample_l = self.detune_delay_left.read_at_lin(working_ml as Sample);
            let input_sample_r = self.detune_delay_right.read_at_lin(working_mr as Sample);
            // - Apply a lowpass filter to the output from the M delay (iirA variable)
            self.iir_al = (self.iir_al * (1.0 - lowpass)) + (input_sample_l * lowpass);
            let input_sample_l = self.iir_al;
            self.iir_ar = (self.iir_ar * (1.0 - lowpass)) + (input_sample_r * lowpass);
            let input_sample_r = self.iir_ar;
            // - Only calculate a new reverb sample once every 4 samples if SR is 44100*4

            // Reverb sample:
            // Set I-L delays for the input + respective feedback from last cycle for the opposite channel (left for right, right for left)
            // BLOCK 0

            for i in 0..4 {
                self.delays_left[i]
                    .write_and_advance((self.feedback[1][i] * regen) + input_sample_l);
            }
            for i in 0..4 {
                self.delays_right[i]
                    .write_and_advance((self.feedback[0][i] * regen) + input_sample_r);
            }

            let mut block_0_l = [0.0; 4];
            for i in 0..4 {
                block_0_l[i] = self.delays_left[i].read();
            }
            let mut block_0_r = [0.0; 4];
            for i in 0..4 {
                block_0_r[i] = self.delays_right[i].read();
            }
            // BLOCK 1

            for i in 0..4 {
                self.delays_left[i + 4].write_and_advance(
                    block_0_l[0 + i]
                        - (block_0_l[(1 + i) % 4]
                            + block_0_l[(2 + i) % 4]
                            + block_0_l[(3 + i) % 4]),
                );
            }
            for i in 0..4 {
                self.delays_right[i + 4].write_and_advance(
                    block_0_r[0 + i]
                        - (block_0_r[(1 + i) % 4]
                            + block_0_r[(2 + i) % 4]
                            + block_0_r[(3 + i) % 4]),
                );
            }

            let mut block_1_l = [0.0; 4];
            for i in 0..4 {
                block_1_l[i] = self.delays_left[i + 4].read();
            }
            let mut block_1_r = [0.0; 4];
            for i in 0..4 {
                block_1_r[i] = self.delays_right[i + 4].read();
            }

            // BLOCK 2

            for i in 0..4 {
                self.delays_left[i + 8].write_and_advance(
                    block_1_l[0 + i]
                        - (block_1_l[(1 + i) % 4]
                            + block_1_l[(2 + i) % 4]
                            + block_1_l[(3 + i) % 4]),
                );
            }
            for i in 0..4 {
                self.delays_right[i + 8].write_and_advance(
                    block_1_r[0 + i]
                        - (block_1_r[(1 + i) % 4]
                            + block_1_r[(2 + i) % 4]
                            + block_1_r[(3 + i) % 4]),
                );
            }

            let mut block_2_l = [0.0; 4];
            for i in 0..4 {
                block_2_l[i] = self.delays_left[i + 8].read();
            }
            let mut block_2_r = [0.0; 4];
            for i in 0..4 {
                block_2_r[i] = self.delays_right[i + 8].read();
            }

            // Set feedback
            for i in 0..4 {
                self.feedback[0][i] = block_2_l[i]
                    - (block_2_l[(1 + i) % 4] + block_2_l[(2 + i) % 4] + block_2_l[(3 + i) % 4]);
            }
            for i in 0..4 {
                self.feedback[1][i] = block_2_r[i]
                    - (block_2_r[(1 + i) % 4] + block_2_r[(2 + i) % 4] + block_2_r[(3 + i) % 4]);
            }

            let input_sample_l: Sample = block_2_l.iter().sum::<Sample>() * 0.125;
            let input_sample_r: Sample = block_2_r.iter().sum::<Sample>() * 0.125;

            // Get the output from I-L delays
            // Set A-D delays to a mixing configuration of the I-L outputs e.g. I - (J+K+L);
            // Same thing for E-H
            // Feedback delays are this same mixing of the outputs of E-H
            // For large sample rates, use linear interpolation to the new value, otherwise the sum of EFGH/8.
            //
            // Apply another lowpass to the reverbed value

            self.iir_bl = (self.iir_bl * (1.0 - lowpass)) + input_sample_l * lowpass;
            let mut input_sample_l = self.iir_bl;
            self.iir_br = (self.iir_br * (1.0 - lowpass)) + (input_sample_r * lowpass);
            let mut input_sample_r = self.iir_br;

            if wet < 1.0 {
                input_sample_l = (input_sample_l * wet) + (dry_sample_l * (1.0 - wet));
                input_sample_r = (input_sample_r * wet) + (dry_sample_r * (1.0 - wet));
            }

            let (_mantissa_l, exp_l) = frexp(input_sample_l as f32);
            let mut fpd_l = self.fpd_l;
            fpd_l ^= fpd_l << 13;
            fpd_l ^= fpd_l >> 17;
            fpd_l ^= fpd_l << 5;
            input_sample_l += (((fpd_l as f64) - (0x7fffffff_u32) as f64)
                * 5.5e-36
                * (2_u64.pow(exp_l + 62) as f64)) as Sample;
            self.fpd_l = fpd_l;

            let (_mantissa_r, exp_r) = frexp(input_sample_r as f32);
            let mut fpd_r = self.fpd_r;
            fpd_r ^= fpd_r << 13;
            fpd_r ^= fpd_r >> 17;
            fpd_r ^= fpd_r << 5;
            input_sample_r += (((fpd_r as f64) - (0x7fffffff_u32) as f64)
                * 5.5e-36
                * (2_u64.pow(exp_r + 62) as f64)) as Sample;
            self.fpd_r = fpd_r;

            *output_l = input_sample_l;
            *output_r = input_sample_r;
        }
        GenState::Continue
    }
}

fn frexp(s: f32) -> (f32, u32) {
    if 0.0 == s {
        return (s, 0);
    } else {
        let lg = s.abs().log2();
        let x = (lg - lg.floor() - 1.0).exp2();
        let exp = lg.floor() + 1.0;
        (s.signum() * x, exp as u32)
    }
}
