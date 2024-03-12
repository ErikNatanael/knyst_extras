type Sample = f64;

#[derive(Clone, Copy, Debug)]
pub struct Allpass {
    coeff: Sample,
    prev_input: Sample,
    prev_output: Sample,
}

impl Allpass {
    pub fn new() -> Self {
        Self {
            coeff: 0.0,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }
    /// Reset any state to 0
    pub fn clear(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }
    pub fn set_delta(&mut self, delta: Sample) {
        self.coeff = (1.0 - delta) / (1.0 + delta);
    }
    pub fn process(&mut self, input: Sample) -> Sample {
        let output = self.coeff * (input - self.prev_output) + self.prev_input;
        self.prev_output = output;
        self.prev_input = input;
        output
    }
}

/// Simple non-feedback allpass delay with linear interpolation between delay time settings
#[derive(Clone, Debug)]
pub struct AllpassDelayLinInterp {
    delay: AllpassDelay,
    target_delay_length_in_frames: f64,
    current_delay_length_in_frames: f64,
    delay_length_step_size: f64,
    delay_length_steps_left: usize,
}
impl AllpassDelayLinInterp {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            delay: AllpassDelay::new(buffer_size),
            target_delay_length_in_frames: 1.,
            delay_length_step_size: 0.,
            delay_length_steps_left: 0,
            current_delay_length_in_frames: 1.,
        }
    }
    pub fn read(&mut self) -> Sample {
        self.delay.read()
    }
    pub fn set_delay_in_frames(&mut self, num_frames: f64) {
        const NUM_FRAMES_TO_INTERPOLATE: usize = 40;
        self.delay_length_steps_left = NUM_FRAMES_TO_INTERPOLATE - 1;
        self.target_delay_length_in_frames = num_frames;
        self.delay_length_step_size =
            (num_frames - self.current_delay_length_in_frames) / NUM_FRAMES_TO_INTERPOLATE as f64;
        self.current_delay_length_in_frames += self.delay_length_step_size;
        self.delay
            .set_delay_in_frames(self.current_delay_length_in_frames);
    }
    pub fn write_and_advance(&mut self, input: Sample) {
        self.delay.write_and_advance(input);
        if self.delay_length_steps_left > 0 {
            self.current_delay_length_in_frames += self.delay_length_step_size;
            self.delay
                .set_delay_in_frames(self.current_delay_length_in_frames);
            self.delay_length_steps_left -= 1;
        }
    }
    pub fn clear(&mut self) {
        self.delay.clear();
    }
}

#[derive(Clone, Debug)]
pub struct AllpassDelay {
    buffer: Vec<Sample>,
    frame: usize,
    num_frames: usize,
    allpass: Allpass,
}

impl AllpassDelay {
    pub fn new(buffer_size: usize) -> Self {
        let buffer = vec![0.0; buffer_size];
        Self {
            buffer,
            frame: 0,
            num_frames: 1,
            allpass: Allpass::new(),
        }
    }
    /// Read the current frame from the delay and allpass interpolate. Read before `write_and_advance` for the correct sample.
    pub fn read(&mut self) -> Sample {
        let index = if self.frame < self.num_frames {
            self.buffer.len() - self.num_frames + self.frame
        } else {
            self.frame - self.num_frames
        };
        self.allpass.process(self.buffer[index])
    }
    pub fn set_delay_in_frames(&mut self, num_frames: f64) {
        self.num_frames = num_frames.floor() as usize;
        self.allpass
            .set_delta((num_frames - self.num_frames as f64) as Sample);
    }
    pub fn clear(&mut self) {
        for sample in &mut self.buffer {
            *sample = 0.0;
        }
        self.allpass.clear();
    }
    /// Reset the delay with a new length in frames
    pub fn set_delay_in_frames_and_clear(&mut self, num_frames: f64) {
        for sample in &mut self.buffer {
            *sample = 0.0;
        }
        self.set_delay_in_frames(num_frames);
        // println!(
        //     "num_frames: {}, delta: {}",
        //     self.num_frames,
        //     (num_frames - self.num_frames as f64)
        // );
    }
    /// Write a new value into the delay after incrementing the sample pointer.
    pub fn write_and_advance(&mut self, input: Sample) {
        self.buffer[self.frame] = input;
        self.frame += 1;
        if self.frame >= self.buffer.len() {
            self.frame = 0;
        }
    }
}

/// Allpass delay (non-feedback) with two taps that are crossfaded between
#[derive(Clone, Debug)]
pub struct AllpassDelayCrossfadeInterp {
    buffer: Vec<Sample>,
    frame: usize,
    num_frames_0: usize,
    num_frames_1: usize,
    allpass_0: Allpass,
    allpass_1: Allpass,
    crossfade_mix: Sample,
    crossfade_step: Sample,
}

impl AllpassDelayCrossfadeInterp {
    pub fn new(buffer_size: usize) -> Self {
        let buffer = vec![0.0; buffer_size];
        Self {
            buffer,
            frame: 0,
            num_frames_0: 1,
            num_frames_1: 1,
            allpass_0: Allpass::new(),
            allpass_1: Allpass::new(),
            crossfade_mix: 0.,
            crossfade_step: -1.0 / 40.,
        }
    }
    /// Read the current frame from the delay and allpass interpolate. Read before `write_and_advance` for the correct sample.
    pub fn read(&mut self) -> Sample {
        self.crossfade_mix += self.crossfade_step;
        if self.crossfade_mix < 0.0 {
            self.crossfade_mix = 0.0;
        } else if self.crossfade_mix > 1.0 {
            self.crossfade_mix = 1.0;
        }
        let index_0 = if self.frame < self.num_frames_0 {
            self.buffer.len() - self.num_frames_0 + self.frame
        } else {
            self.frame - self.num_frames_0
        };
        let tap_0 = self.allpass_0.process(self.buffer[index_0]);
        let index_1 = if self.frame < self.num_frames_1 {
            self.buffer.len() - self.num_frames_1 + self.frame
        } else {
            self.frame - self.num_frames_1
        };
        let tap_1 = self.allpass_1.process(self.buffer[index_1]);
        tap_0 * (1.0 - self.crossfade_mix) + tap_1 * self.crossfade_mix
    }
    pub fn set_delay_in_frames(&mut self, num_frames: f64) {
        if self.crossfade_step > 0.0 {
            // tap 1 is the main tap, change tap 0
            self.num_frames_0 = num_frames.floor() as usize;
            self.allpass_0
                .set_delta((num_frames - self.num_frames_0 as f64) as Sample);
        } else {
            // tap 0 is the main tap, change tap 1
            self.num_frames_1 = num_frames.floor() as usize;
            self.allpass_1
                .set_delta((num_frames - self.num_frames_1 as f64) as Sample);
        }
        self.crossfade_step *= -1.;
    }
    pub fn clear(&mut self) {
        for sample in &mut self.buffer {
            *sample = 0.0;
        }
        self.allpass_0.clear();
        self.allpass_1.clear();
    }
    /// Reset the delay with a new length in frames
    pub fn set_delay_in_frames_and_clear(&mut self, num_frames: f64) {
        for sample in &mut self.buffer {
            *sample = 0.0;
        }
        self.set_delay_in_frames(num_frames);
        // println!(
        //     "num_frames: {}, delta: {}",
        //     self.num_frames,
        //     (num_frames - self.num_frames as f64)
        // );
    }
    /// Write a new value into the delay after incrementing the sample pointer.
    pub fn write_and_advance(&mut self, input: Sample) {
        self.buffer[self.frame] = input;
        self.frame += 1;
        if self.frame >= self.buffer.len() {
            self.frame = 0;
        }
    }
}

#[derive(Clone, Debug)]
pub struct AllpassFeedbackDelay {
    pub feedback: Sample,
    allpass_delay: AllpassDelayLinInterp,
}
impl AllpassFeedbackDelay {
    pub fn new(max_delay_samples: usize) -> Self {
        let allpass_delay = AllpassDelayLinInterp::new(max_delay_samples);
        let s = Self {
            feedback: 0.,
            allpass_delay,
        };
        s
    }
    pub fn set_delay_in_frames(&mut self, delay_length: f64) {
        self.allpass_delay.set_delay_in_frames(delay_length);
    }
    /// Clear any values in the delay
    pub fn clear(&mut self) {
        self.allpass_delay.clear();
    }
    // fn calculate_values(&mut self) {
    //     self.feedback = (0.001 as Sample).powf(self.delay_time / self.decay_time.abs())
    //         * self.decay_time.signum();
    //     let delay_samples = self.delay_time * self.sample_rate;
    //     self.allpass_delay.set_num_frames(delay_samples as f64);
    // }
    pub fn process(&mut self, input: Sample) -> Sample {
        let delayed_sig = self.allpass_delay.read();
        if delayed_sig.is_nan() {
            dbg!(&self);
            panic!("nan in allpass");
        }
        let delay_write = delayed_sig * self.feedback + input;
        self.allpass_delay.write_and_advance(delay_write);

        delayed_sig - self.feedback * delay_write
    }
}
