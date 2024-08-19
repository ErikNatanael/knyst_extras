use knyst::{gen::filter::one_pole::OnePole, prelude::*, trig::is_trigger};

const BOW_WAVETABLE_SIZE: usize = 4096;

use crate::AllpassFeedbackDelay;
// This waveguide implementation mirrors the one outlined in Palle Dahlstedt's
//  "Physical Interactions with Digital Strings - A hybrid approach to a digital keyboard instrument"
// It allows you to stop the string to some variable degree.

/// Waveguide gen for the internal delay line implementation
/// *inputs*
/// 0. "exciter": Excitation signal
/// 1. "freq": frequency of the delay line
/// 2. "position": the position of the excitation
/// 3. "feedback": feedback amount
/// *outputs*
/// 0. "sig": output signal
#[derive(Clone, Debug)]
pub struct BowedWaveguideSimplified {
    // one backwards and one forwards delay enables us setting the position of the excitation input signal
    delays: [AllpassFeedbackDelay; 2],
    last_delay_outputs: [f64; 2],
    last_freq: Sample,
    last_position: Sample,
    last_damping: Sample,
    last_lf_damping: Sample,
    lp_filter: [OnePole<f64>; 2],
    hp_filter: [OnePole<f64>; 1],
    lp_filter_delay_compensation: f64,
    exciter_peak_follower: f64,
    bow: Bow,
}

impl BowedWaveguideSimplified {
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
        for sample in self.last_delay_outputs.iter_mut() {
            *sample = 0.0;
        }
    }
    pub fn set_damping(&mut self, damping: f64, high_pass_damping: f64, sample_rate: f64) {
        let damping = damping.clamp(0.0, 20000.);
        for i in 0..2 {
            self.lp_filter[i].set_freq_lowpass(damping, sample_rate);
        }
        self.hp_filter[0].set_freq_highpass(high_pass_damping, sample_rate);

        // We need to compensate more when there's a stop because when the string is stopped the compensation is carried by fewer delay lines
        self.lp_filter_delay_compensation =
            self.lp_filter[0].cheap_tuning_compensation_lpf() * (-1.0); // - stop_amount * 0.1);
        if self.lp_filter_delay_compensation.is_nan() {
            dbg!(
                damping,
                sample_rate,
                self.lp_filter_delay_compensation,
                self.lp_filter
            );
            panic!("lp delay comp is nan");
        }
        // self.lp_filter_delay_compensation = OnePole::phase_delay(freq, damping) * 2.0;
        // POLL.store(
        //     self.lp_filter_delay_compensation as f32,
        //     std::sync::atomic::Ordering::SeqCst,
        // );
    }
    pub fn set_freq_pos(
        &mut self,
        freq: f64,
        damping: f64,
        position: f64,
        sample_rate: f64,
        delay_compensation: f64,
    ) {
        let freq = freq * 2.0;

        // let freq_ratio = (freq / damping).powi(1) * 0.15;
        let (mut delay0_time, mut delay1_time) = delay_times(freq, position);
        // Why is it 1.5 and not 1.0? Idk, but it keeps the top pitches in tune without the lp filter
        static FEEDBACK_DELAY_COMPENSATION: f64 = 0.125;
        // Make sure there cannot be a negative time delay
        delay0_time = delay0_time * sample_rate + delay_compensation + FEEDBACK_DELAY_COMPENSATION;
        // + self.lp_filter_delay_compensation;
        delay1_time = delay1_time * sample_rate + delay_compensation + FEEDBACK_DELAY_COMPENSATION;
        // + self.lp_filter_delay_compensation;
        if delay0_time.min(delay1_time) < 0.0 {
            delay0_time = (delay0_time + delay1_time).max(0.0);
            delay1_time = 0.0;
            // dbg!(delay0_time, delay0_time);
        }
        if delay0_time.is_nan() || delay1_time.is_nan() {
            dbg!(
                delay0_time,
                delay1_time,
                freq,
                position,
                delay_compensation,
                self.lp_filter_delay_compensation
            );
        }
        self.delays[0].set_delay_in_frames(delay0_time + self.lp_filter_delay_compensation - 1.);
        self.delays[1].set_delay_in_frames(delay1_time);
        // self.delays[2].set_delay_in_frames(delay1_time);
        // self.delays[3].set_delay_in_frames(delay0_time + self.lp_filter_delay_compensation);
        // self.dc_blocker.set_freq_lowpass(30.0, sample_rate);
    }
    pub fn process_sample(
        &mut self,
        exciter_input: f64,
        feedback: f64,
        bow_force: f64,
        bow_velocity: f64,
    ) -> Sample {
        if exciter_input > self.exciter_peak_follower {
            self.exciter_peak_follower = exciter_input;
        } else {
            self.exciter_peak_follower *= 0.95;
        }

        // Calculate bow sample
        let bow_sig = if bow_velocity > 0.0 {
            let velocity = bow_velocity - (self.last_delay_outputs[0] * 2.);
            self.bow.slope = 5.0 - (4.0 * bow_force);

            velocity * self.bow.process_sample(velocity)
        } else {
            0.0
        };
        // let bow_index = ((velocity.abs() + bow_force.abs() * 0.2).clamp(0.0, 1.0)
        //     * (BOW_WAVETABLE_SIZE - 1) as f64) as usize;
        // let bow_sig = self.bow_wavetable[bow_index] * bow_force * velocity;

        // let bow_index = velocity.abs();
        // let bow_sig = if bow_index < bow_force || bow_force >= 1.0 {
        //     1.0
        // } else if bow_index > 1.0 {
        //     0.0
        // } else {
        //     (bow_index - bow_force) / (1.0 - bow_force)
        // };

        let exciter_input = exciter_input + bow_sig;
        let mut sig = 0.0;
        for i in 0..2 {
            let prev_node_index = if i == 0 { 1 } else { i - 1 };
            let cross_delay_feedback = self.last_delay_outputs[prev_node_index];
            if cross_delay_feedback.is_nan() {
                dbg!(i, cross_delay_feedback);
                panic!("NaN in cross");
            }
            let mut segment_sig = cross_delay_feedback;
            // Put the delayed signal through an LPF and apply the relevant coefficient
            // let delay_input = (cross_delay_feedback).tanh();
            if i == 0 || i == 2 {
                // nut/bridge
                // phase shift 180degrees
                segment_sig *= -1. * feedback;
                segment_sig = self.lp_filter[prev_node_index].process_lp(segment_sig);
                if segment_sig.is_nan() {
                    dbg!(i, segment_sig);
                    panic!("NaN in ");
                }
                if segment_sig.is_nan() {
                    dbg!(i, feedback, segment_sig);
                    panic!("NaN in ");
                }
                segment_sig = non_linearity(segment_sig);
            } else {
                // previous open string segment + excitation signal + previous stopped string segment
                let previous_open_string_segment = segment_sig;
                // input amount depends on how stopped the string is

                segment_sig = previous_open_string_segment + exciter_input;
                if segment_sig.is_nan() {
                    dbg!(
                        i,
                        previous_open_string_segment,
                        exciter_input,
                        bow_sig,
                        bow_force
                    );
                    panic!("NaN in ");
                }
            }

            let delay_input = segment_sig;

            if delay_input.is_nan() {
                dbg!(i, segment_sig, delay_input);
                panic!("NaN in ");
            }
            // let delay_input = cross_delay_feedback;
            let mut delay_output = self.delays[i].process(delay_input);
            if delay_output.is_nan() {
                dbg!(i, delay_output, delay_input);
                panic!("NaN in ");
            }
            if i == 0 {
                // After Delay0, tap the signal and apply a DC blocker
                sig += delay_output;
                delay_output = self.hp_filter[0].process_hp(delay_output);
            }
            self.last_delay_outputs[i] = delay_output;
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
impl BowedWaveguideSimplified {
    pub fn new() -> Self {
        Self {
            delays: [
                AllpassFeedbackDelay::new(192000 / 20),
                AllpassFeedbackDelay::new(192000 / 20),
            ],
            last_delay_outputs: [0.0; 2],
            last_freq: 0.0,
            last_position: 0.0,
            last_damping: 0.0,
            last_lf_damping: 0.0,
            lp_filter: [OnePole::new(); 2],
            hp_filter: [OnePole::new()],
            lp_filter_delay_compensation: 0.0,
            exciter_peak_follower: 0.,
            bow: Bow::new(),
        }
    }
    pub fn init(&mut self, sample_rate: SampleRate) {
        let max_delay_size = 16384; // TODO: set to the next higher power of 2 up from sample_rate/20
        *self = Self {
            delays: [
                AllpassFeedbackDelay::new(max_delay_size),
                AllpassFeedbackDelay::new(max_delay_size),
            ],
            last_delay_outputs: [0.0; 2],
            last_freq: 0.0,
            last_position: 0.0,
            last_damping: 0.0,
            last_lf_damping: 0.0,
            lp_filter: [OnePole::new(); 2],
            hp_filter: [OnePole::new()],
            lp_filter_delay_compensation: 0.0,
            exciter_peak_follower: 0.,
            bow: Bow::new(),
        };
    }
    pub fn process(
        &mut self,
        exciter: &[Sample],
        freq: &[Sample],
        position: &[Sample],
        feedback: &[Sample],
        stiffness: &[Sample],
        damping: &[Sample],
        lf_damping: &[Sample],
        delay_compensation: &[Sample],
        bow_force: &[Sample],
        bow_velocity: &[Sample],
        reset_trig: &[Sample],
        output: &mut [Sample],
        sample_rate: SampleRate,
    ) -> GenState {
        let sample_rate = *sample_rate;
        let exciter_buf = exciter;
        for (
            (
                (
                    (
                        (
                            (
                                (
                                    ((((&exciter, &freq), &position), &feedback), &stiffness),
                                    &damping,
                                ),
                                &lf_damping,
                            ),
                            &delay_comp,
                        ),
                        &reset_trig,
                    ),
                    &bow_force,
                ),
                &bow_velocity,
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
            .zip(bow_force)
            .zip(bow_velocity)
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
                    damping as f64,
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
            // let stop_amount = smootherstep(0.0, 1.0, stop_amount as f64);
            *output = self.process_sample(
                exciter as f64,
                feedback as f64,
                bow_force as f64,
                bow_velocity as f64,
            );
            if output.is_nan() {
                dbg!(
                    exciter,
                    exciter_buf,
                    freq,
                    position,
                    damping,
                    lf_damping,
                    sample_rate,
                    delay_comp,
                    reset_trig,
                    stiffness,
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

fn smootherstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    // Scale, and clamp x to 0..1 range
    let x = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);

    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

#[derive(Copy, Clone, Debug)]
struct Bow {
    slope: f64,
    offset: f64,
    min: f64,
    max: f64,
}
impl Bow {
    pub fn new() -> Self {
        Self {
            slope: 0.0,
            offset: 0.0,
            min: 0.01,
            max: 0.98,
        }
    }
    fn process_sample(&mut self, delta_velocity: f64) -> f64 {
        let sig = delta_velocity + self.offset;
        let sig = sig * self.slope;
        let sig = sig.abs() + 0.75;
        let sig = sig.powi(-4);
        sig.clamp(self.min, self.max)
    }
}
