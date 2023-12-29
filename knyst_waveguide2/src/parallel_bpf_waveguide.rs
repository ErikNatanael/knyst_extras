use crate::delay_times;
use biquad::{Biquad, ToHertz};
use knyst::prelude::*;
use knyst::trig::is_trigger;
use knyst::{gen::GenState, Sample, SampleRate};

use super::delay::*;
use knyst::gen::filter::one_pole::*;

/// Waveguide gen for the internal delay line implementation
/// *inputs*
/// 0. "exciter": Excitation signal
/// 1. "freq": frequency of the delay line
/// 2. "position": the position of the excitation
/// 3. "feedback": feedback amount
/// *outputs*
/// 0. "sig": output signal
pub struct ParallelBpfWaveguide {
    // one backwards and one forwards delay enables us setting the position of the excitation input signal
    delays: [AllpassFeedbackDelay; 2],
    last_delay_outputs: [f64; 2],
    last_freq: Sample,
    last_position: Sample,
    last_damping: Sample,
    last_lf_damping: Sample,
    dc_blocker: [OnePole<f64>; 1],
    lp_filter: [OnePole<f64>; 1],
    hp_filter: [OnePole<f64>; 1],
    parallel_filter: biquad::DirectForm1<f64>,
    sample_rate: f64,
    bpf_freq: f64,
    lp_filter_delay_compensation: f64,
}

impl ParallelBpfWaveguide {
    pub fn reset(&mut self) {
        // dbg!("Reset", self.last_delay_outputs);
        for delay in &mut self.delays {
            delay.clear();
        }
        for filter in &mut self.lp_filter {
            filter.reset();
        }
        for filter in &mut self.hp_filter {
            filter.reset();
        }
        for filter in &mut self.dc_blocker {
            filter.reset();
        }
        self.last_delay_outputs[0] = 0.0;
        self.last_delay_outputs[1] = 0.0;
    }
    pub fn set_damping(&mut self, damping: f64, high_pass_damping: f64, sample_rate: f64) {
        for i in 0..1 {
            self.lp_filter[i].set_freq_lowpass(damping, sample_rate);
            self.hp_filter[i].set_freq_highpass(high_pass_damping, sample_rate);
            // TODO: DC blocker HPF doesn't work
            self.dc_blocker[i].set_freq_highpass(30.0, sample_rate);
        }

        self.lp_filter_delay_compensation =
            self.lp_filter[0].cheap_tuning_compensation_lpf() * -0.5;
        // self.lp_filter_delay_compensation = OnePole::phase_delay(freq, damping) * 2.0;
        // POLL.store(
        //     self.lp_filter_delay_compensation as f32,
        //     std::sync::atomic::Ordering::SeqCst,
        // );
    }
    pub fn set_freq_pos(&mut self, freq: f64, position: f64, sample_rate: f64) {
        let (mut delay0_time, mut delay1_time) = delay_times(freq, position);
        // Why is it 1.5 and not 1.0? Idk, but it keeps the top pitches in tune without the lp filter
        static FEEDBACK_DELAY_COMPENSATION: f64 = 1.5;
        // Make sure there cannot be a negative time delay
        delay0_time = delay0_time * sample_rate - FEEDBACK_DELAY_COMPENSATION
            + self.lp_filter_delay_compensation;
        delay1_time = delay1_time * sample_rate - FEEDBACK_DELAY_COMPENSATION
            + self.lp_filter_delay_compensation;
        if delay0_time.min(delay1_time) < 0.0 {
            delay0_time = (delay0_time + delay1_time).max(0.0);
            delay1_time = 0.0;
            // dbg!(delay0_time, delay0_time);
        }
        self.delays[0].set_delay_in_frames(delay0_time);
        self.delays[1].set_delay_in_frames(delay1_time);
        // self.dc_blocker.set_freq_lowpass(30.0, sample_rate);
    }
    pub fn set_bpf_freq(&mut self, bpf_freq: f64) {
        let coeffs = biquad::Coefficients::<f64>::from_params(
            biquad::Type::BandPass,
            self.sample_rate.hz(),
            bpf_freq.hz(),
            // biquad::Q_BUTTERWORTH_F64,
            5.0,
        )
        .unwrap();
        self.parallel_filter.replace_coefficients(coeffs);
        self.bpf_freq = bpf_freq;
    }
    pub fn process_sample(&mut self, exciter_input: f64, feedback: f64, bpf_mix: f64) -> Sample {
        let mut sig = 0.0;
        for i in 0..2 {
            let cross_delay_feedback = self.last_delay_outputs[1 - i];
            // let delay_input = (cross_delay_feedback).tanh();
            let delay_input = non_linearity(cross_delay_feedback);
            // let delay_input = cross_delay_feedback;
            let delay_output = self.delays[i].process(delay_input);
            let inner_sig = delay_output + exciter_input;
            // TODO: DC blocker HPF doesn't work
            // let inner_sig = self.dc_blocker[i].process(inner_sig);
            if i == 0 {
                let inner_sig = self.lp_filter[0].process_lp(inner_sig);
                let inner_sig = self.hp_filter[0].process_hp(inner_sig);
                let bpf = self.parallel_filter.run(inner_sig);
                let inner_sig = inner_sig * (1.0 - bpf_mix) + bpf * bpf_mix;
                self.last_delay_outputs[i] = inner_sig * feedback * -1.;
                sig += inner_sig;
                // sig += inner_sig * 2.0;
            } else {
                self.last_delay_outputs[i] = inner_sig * feedback * -1.;
                sig += inner_sig;
            }
        }
        sig as Sample
    }
}
fn non_linearity(x: f64) -> f64 {
    let x = x.clamp(-2.0, 2.0);
    // (x - (x.powi(3) / 3.)) * 1.5
    x - (x.powi(3) / 3.)
}
#[impl_gen]
impl ParallelBpfWaveguide {
    pub fn new() -> Self {
        let center_freq = 500.hz();
        let fs = 44100.hz();
        let coeffs = biquad::Coefficients::<f64>::from_params(
            biquad::Type::BandPass,
            fs,
            center_freq,
            biquad::Q_BUTTERWORTH_F64,
        )
        .unwrap();
        Self {
            delays: [
                AllpassFeedbackDelay::new(192000 / 20),
                AllpassFeedbackDelay::new(192000 / 20),
            ],
            last_delay_outputs: [0.0, 0.0],
            last_freq: 0.0,
            last_position: 0.0,
            last_damping: 0.0,
            last_lf_damping: 0.0,
            dc_blocker: [OnePole::new()],
            lp_filter: [OnePole::new()],
            hp_filter: [OnePole::new()],
            lp_filter_delay_compensation: 0.0,
            parallel_filter: biquad::DirectForm1::<f64>::new(coeffs),
            sample_rate: 44100.,
            bpf_freq: 200.,
        }
    }
    fn init(&mut self, sample_rate: SampleRate) {
        let center_freq = 500.hz();
        let fs = sample_rate.hz();
        let coeffs = biquad::Coefficients::<f64>::from_params(
            biquad::Type::BandPass,
            fs,
            center_freq,
            4.0,
        )
        .unwrap();
        *self = Self {
            delays: [
                AllpassFeedbackDelay::new(sample_rate.to_usize() / 20),
                AllpassFeedbackDelay::new(sample_rate.to_usize() / 20),
            ],
            last_delay_outputs: [0.0, 0.0],
            last_freq: 0.0,
            last_position: 0.0,
            last_damping: 0.0,
            last_lf_damping: 0.0,
            dc_blocker: [OnePole::new()],
            lp_filter: [OnePole::new()],
            hp_filter: [OnePole::new()],
            lp_filter_delay_compensation: 0.0,
            sample_rate: *sample_rate as f64,
            parallel_filter: biquad::DirectForm1::<f64>::new(coeffs),
            bpf_freq: 200.,
        };
    }
    fn process(
        &mut self,
        exciter: &[Sample],
        freq: &[Sample],
        position: &[Sample],
        feedback: &[Sample],
        stiffness: &[Sample],
        damping: &[Sample],
        lf_damping: &[Sample],
        bpf_freq: &[Sample],
        bpf_mix: &[Sample],
        reset_trig: &[Sample],
        output: &mut [Sample],
        sample_rate: SampleRate,
    ) -> GenState {
        let sample_rate = *sample_rate;
        for (
            (
                (
                    (
                        (
                            (((((&exciter, &freq), &position), &feedback), &stiffness), &damping),
                            &lf_damping,
                        ),
                        &bpf_freq,
                    ),
                    &bpf_mix,
                ),
                &reset_trig,
            ),
            output,
        ) in exciter
            .iter()
            .zip(freq)
            .zip(position)
            .zip(feedback)
            .zip(stiffness)
            .zip(damping)
            .zip(lf_damping)
            .zip(bpf_freq)
            .zip(bpf_mix)
            .zip(reset_trig)
            .zip(output.iter_mut())
        {
            if is_trigger(reset_trig) {
                self.reset();
            }
            let damping_changed =
                if damping != self.last_damping || self.last_lf_damping != lf_damping {
                    self.set_damping(damping as f64, lf_damping as f64, sample_rate as f64);
                    self.last_damping = damping;
                    self.last_lf_damping = lf_damping;
                    true
                } else {
                    false
                };
            if damping_changed || freq != self.last_freq || position != self.last_position {
                let freq = freq.max(20.);
                self.set_freq_pos(freq as f64, position as f64, sample_rate as f64);
                self.last_freq = freq;
                self.last_position = position;
            }
            if bpf_freq as f64 != self.bpf_freq {
                self.set_bpf_freq(bpf_freq as f64);
            }
            for i in 0..2 {
                self.delays[i].feedback = stiffness as f64;
            }
            *output = self.process_sample(exciter as f64, feedback as f64, bpf_mix as f64);
            if output.is_nan() {
                dbg!(
                    freq,
                    position,
                    damping,
                    lf_damping,
                    sample_rate,
                    reset_trig,
                    stiffness,
                    bpf_freq,
                    bpf_mix,
                );
                panic!("NaN in waveguide.");
            }
        }
        // dbg!(&output_buf);
        GenState::Continue
    }
}
