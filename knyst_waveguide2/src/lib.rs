//! Rewrite of the knyst_waveguide lib
//!
//! Instead of lots of custom structs, create a Handle type with methods to set the important parameters, and an init function that returns this handle..
//!

pub mod bowed_string;
mod delay;
pub mod double_buffer_waveguide;
pub mod parallel_bpf_waveguide;
pub mod split_string;
use std::f32::consts::{PI, TAU};

use delay::*;
use knyst::gen::filter::one_pole::*;
use knyst::trig::is_trigger;
use knyst::wavetable::WavetablePhase;
use knyst::xorrng::XOrShift32Rng;
use knyst::Sample;
use knyst::{prelude::*, wavetable::FRACTIONAL_PART};

/// Waveguide gen for the internal delay line implementation
/// *inputs*
/// 0. "exciter": Excitation signal
/// 1. "freq": frequency of the delay line
/// 2. "position": the position of the excitation
/// 3. "feedback": feedback amount
/// *outputs*
/// 0. "sig": output signal
pub struct Waveguide {
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
    lp_filter_delay_compensation: f64,
}

impl Waveguide {
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
    pub fn set_freq_pos(
        &mut self,
        freq: f64,
        position: f64,
        sample_rate: f64,
        delay_compensation: f64,
    ) {
        let (mut delay0_time, mut delay1_time) = delay_times(freq, position);
        // Why is it 1.5 and not 1.0? Idk, but it keeps the top pitches in tune without the lp filter
        static FEEDBACK_DELAY_COMPENSATION: f64 = 1.5;
        // Make sure there cannot be a negative time delay
        delay0_time = delay0_time * sample_rate - FEEDBACK_DELAY_COMPENSATION
            + delay_compensation
            + self.lp_filter_delay_compensation;
        delay1_time = delay1_time * sample_rate - FEEDBACK_DELAY_COMPENSATION
            + delay_compensation
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
    pub fn process_sample(&mut self, exciter_input: f64, feedback: f64) -> Sample {
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
impl Waveguide {
    pub fn new() -> Self {
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
        }
    }
    fn init(&mut self, sample_rate: SampleRate) {
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
        delay_compensation: &[Sample],
        reset_trig: &[Sample],
        output: &mut [Sample],
        sample_rate: SampleRate,
    ) -> GenState {
        let sample_rate = *sample_rate;
        for (
            (
                (
                    (
                        (((((&exciter, &freq), &position), &feedback), &stiffness), &damping),
                        &lf_damping,
                    ),
                    &delay_comp,
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
            .zip(delay_compensation)
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
                self.set_freq_pos(
                    freq as f64,
                    position as f64,
                    sample_rate as f64,
                    delay_comp as f64,
                );
                self.last_freq = freq;
                self.last_position = position;
            }
            for i in 0..2 {
                self.delays[i].feedback = stiffness as f64;
            }
            *output = self.process_sample(exciter as f64, feedback as f64);
            if output.is_nan() {
                dbg!(
                    freq,
                    position,
                    damping,
                    lf_damping,
                    sample_rate,
                    delay_comp,
                    reset_trig,
                    stiffness
                );
                panic!("NaN in waveguide.");
            }
        }
        // dbg!(&output_buf);
        GenState::Continue
    }
}

fn delay_times(freq: f64, position: f64) -> (f64, f64) {
    let total_delay = freq.recip();
    let time0 = total_delay * position;
    let time1 = total_delay - time0;
    (time0, time1)
}

pub struct WhiteNoise {
    noise: dasp::signal::Noise,
}
impl Default for WhiteNoise {
    fn default() -> Self {
        Self::new()
    }
}

#[impl_gen]
impl WhiteNoise {
    pub fn new() -> Self {
        Self {
            noise: dasp::signal::noise(10),
        }
    }
    fn process(&mut self, output: &mut [Sample]) -> GenState {
        for out in output {
            *out = self.noise.next_sample() as Sample;
        }
        GenState::Continue
    }
}

pub struct HalfSineImpulse {
    finished: bool,
    phase: f32,
}

impl HalfSineImpulse {
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.finished = false;
    }
    pub fn next_sample(&mut self, freq: f32, sample_rate: f32) -> Sample {
        if self.finished {
            return 0.0;
        }
        let out = self.phase.cos();
        self.phase += (TAU * freq) / (sample_rate);
        if self.phase >= PI {
            self.finished = true;
        }
        out
    }
}

#[impl_gen]
impl HalfSineImpulse {
    pub fn new() -> Self {
        Self {
            finished: true,
            phase: 0.0,
        }
    }
    fn process(
        &mut self,
        freq: &[Sample],
        amp: &[Sample],
        restart: &[Trig],
        sig: &mut [Sample],
        sample_rate: SampleRate,
    ) -> GenState {
        for (((&freq, &amp), &restart_trig), out) in
            freq.iter().zip(amp).zip(restart).zip(sig.iter_mut())
        {
            if is_trigger(restart_trig) {
                self.reset();
            }
            *out = self.next_sample(freq, *sample_rate) * amp;
        }
        GenState::Continue
    }
}
/// Oscillator using a shared [`Wavetable`] stored in a [`Resources`]. Assumes the wavetable has normal range for the `range` method on the Handle.
/// *inputs*
/// 0. "freq": Frequency of oscillation
/// *outputs*
/// 0. "sig": Output signal
#[derive(Debug, Clone)]
pub struct HalfSineWt {
    step: u32,
    phase: WavetablePhase,
    wavetable: IdOrKey<WavetableId, WavetableKey>,
    freq_to_phase_inc: f64,
    finished: bool,
}

#[allow(missing_docs)]
#[impl_gen(range = normal)]
impl HalfSineWt {
    #[new]
    #[must_use]
    pub fn new() -> Self {
        let wavetable = IdOrKey::Id(WavetableId::cos());
        Self {
            step: 0,
            phase: WavetablePhase(0),
            wavetable: wavetable.into(),
            freq_to_phase_inc: 0.,
            finished: true,
        }
    }
    #[inline]
    pub fn set_freq(&mut self, freq: Sample) {
        self.step = (freq as f64 * self.freq_to_phase_inc) as u32;
    }
    #[inline]
    pub fn reset_phase(&mut self) {
        self.phase.0 = 0;
    }
    pub fn reset(&mut self) {
        self.reset_phase();
        self.finished = false;
    }
    #[process]
    pub fn process(
        &mut self,
        freq: &[Sample],
        amp: &[Sample],
        restart: &[Trig],
        sig: &mut [Sample],
        resources: &mut Resources,
    ) -> GenState {
        let wt_key = match self.wavetable {
            IdOrKey::Id(id) => {
                if let Some(key) = resources.wavetable_key_from_id(id) {
                    self.wavetable = IdOrKey::Key(key);
                    key
                } else {
                    sig.fill(0.0);
                    return GenState::Continue;
                }
            }
            IdOrKey::Key(key) => key,
        };
        if let Some(wt) = resources.wavetable(wt_key) {
            for (((&f, &restart_trig), &amp), o) in
                freq.iter().zip(restart).zip(amp).zip(sig.iter_mut())
            {
                self.set_freq(f);
                if is_trigger(restart_trig) {
                    self.reset();
                }
                if self.finished {
                    *o = 0.0;
                } else {
                    // TODO: Set a buffer of phase values and request them all from the wavetable all at the same time. Should enable SIMD in the wavetable lookup.
                    *o = wt.get_linear_interp(self.phase) * amp;
                    self.phase.increase(self.step);
                    if self.phase.integer_component() > TABLE_SIZE / 2 {
                        self.finished = true;
                    }
                }
            }
        } else {
            sig.fill(0.0);
        }
        GenState::Continue
    }
    #[init]
    pub fn init(&mut self, sample_rate: SampleRate) {
        self.freq_to_phase_inc =
            TABLE_SIZE as f64 * f64::from(FRACTIONAL_PART) * (1.0 / f64::from(sample_rate));
    }
}

pub struct XorNoise(XOrShift32Rng);

impl XorNoise {}

#[impl_gen]
impl XorNoise {
    pub fn new(seed: u32) -> Self {
        Self(XOrShift32Rng::new(seed))
    }
    fn process(&mut self, noise: &mut [Sample]) -> GenState {
        for o in noise {
            *o = self.0.gen_f32();
        }
        GenState::Continue
    }
}
