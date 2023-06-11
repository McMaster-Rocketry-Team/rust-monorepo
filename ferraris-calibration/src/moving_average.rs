use core::ops::{AddAssign, Div, SubAssign};

use heapless::Deque;

// Adapted from https://docs.rs/simple_moving_average/latest/src/simple_moving_average/single_sum_sma.rs.html
pub(crate) struct SingleSumSMA<Sample, const WINDOW_SIZE: usize> {
    samples: Deque<Sample, WINDOW_SIZE>,
    sum: Sample,
    zero: Sample,
}

impl<Sample, const WINDOW_SIZE: usize> SingleSumSMA<Sample, WINDOW_SIZE>
where
    Sample: Copy + AddAssign + SubAssign + Div<f64, Output = Sample> + core::fmt::Debug,
{
    pub fn new(zero: Sample) -> Self {
        Self {
            samples: Deque::new(),
            sum: zero,
            zero,
        }
    }

    pub fn add_sample(&mut self, new_sample: Sample) {
        if WINDOW_SIZE == 0 {
            return;
        }

        self.sum += new_sample;

        if self.samples.is_full() {
            self.sum -= self.samples.pop_back().unwrap();
        }
        self.samples.push_front(new_sample).unwrap();
    }

    pub fn get_average(&self) -> Sample {
        let num_samples = self.samples.len();

        if num_samples == 0 {
            return self.sum;
        }

        self.sum / num_samples as f64
    }

    pub fn is_full(&self) -> bool {
        self.samples.is_full()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
        self.sum = self.zero;
    }
}
