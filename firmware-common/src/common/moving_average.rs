use core::{
    marker::PhantomData,
    ops::{AddAssign, Div, SubAssign},
};

use heapless::Deque;
use num_traits::FromPrimitive;

// Adapted from https://docs.rs/simple_moving_average/latest/src/simple_moving_average/no_sum_sma.rs.html
pub struct NoSumSMA<Sample, Divisor: FromPrimitive, const WINDOW_SIZE: usize>
where
    Divisor: FromPrimitive,
    Sample: Copy + AddAssign + SubAssign + Div<Divisor, Output = Sample> + core::fmt::Debug,
{
    samples: Deque<Sample, WINDOW_SIZE>,
    zero: Sample,
    divisor: PhantomData<Divisor>,
}

impl<Sample, Divisor, const WINDOW_SIZE: usize> NoSumSMA<Sample, Divisor, WINDOW_SIZE>
where
    Divisor: FromPrimitive,
    Sample: Copy + AddAssign + SubAssign + Div<Divisor, Output = Sample> + core::fmt::Debug,
{
    pub fn new(zero: Sample) -> Self {
        Self {
            samples: Deque::new(),
            zero,
            divisor: PhantomData,
        }
    }

    pub fn add_sample(&mut self, new_sample: Sample) {
        if WINDOW_SIZE == 0 {
            return;
        }

        if self.samples.is_full() {
            self.samples.pop_back().unwrap();
        }
        self.samples.push_front(new_sample).unwrap();
    }

    pub fn get_average(&self) -> Sample {
        let num_samples = self.samples.len();

        if num_samples == 0 {
            return self.zero;
        }

        let mut sum = self.zero;

        for sample in self.samples.iter() {
            sum += *sample;
        }

        sum / Divisor::from_usize(num_samples).unwrap()
    }

    pub fn is_full(&self) -> bool {
        self.samples.is_full()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}
