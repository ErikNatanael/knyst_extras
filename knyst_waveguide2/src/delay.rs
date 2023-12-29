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

#[derive(Clone, Debug)]
pub struct AllpassDelay {
    buffer: Vec<Sample>,
    buffer_size: usize,
    frame: usize,
    num_frames: usize,
    allpass: Allpass,
}

impl AllpassDelay {
    pub fn new(buffer_size: usize) -> Self {
        let buffer = vec![0.0; buffer_size];
        Self {
            buffer,
            buffer_size,
            frame: 0,
            num_frames: 1,
            allpass: Allpass::new(),
        }
    }
    /// Read the current frame from the delay and allpass interpolate. Read before `write_and_advance` for the correct sample.
    pub fn read(&mut self) -> Sample {
        let index = self.frame % self.buffer.len();
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
        self.frame += 1;
        let index = (self.frame + self.num_frames) % self.buffer_size;
        self.buffer[index] = input;
    }
}

#[derive(Clone, Debug)]
pub struct AllpassFeedbackDelay {
    pub feedback: Sample,
    allpass_delay: AllpassDelay,
}
impl AllpassFeedbackDelay {
    pub fn new(max_delay_samples: usize) -> Self {
        let allpass_delay = AllpassDelay::new(max_delay_samples);
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
        let delay_write = delayed_sig * self.feedback + input;
        self.allpass_delay.write_and_advance(delay_write);

        delayed_sig - self.feedback * delay_write
    }
}
