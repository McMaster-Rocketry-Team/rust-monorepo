use core::{array, intrinsics};

use biquad::*;
use cmsis_dsp::transform::FloatRealFft;
use embassy_sync::blocking_mutex::raw::RawMutex;
use heapless::Vec;

use crate::driver::sg_adc::RawSGReadingsTrait;

use super::{
    global_states::SGGlobalStates, ProcessedSGReading, BATCH_SIZE, READS_PER_SECOND,
    SAMPLES_PER_READ,
};

const FFT_SIZE: usize = 2048;

struct SGChannelProcessor<'a> {
    // samples used for fft
    fft_samples_sum: [f32; FFT_SIZE],
    fft_samples_sum_count: usize,
    fft_i: usize,
    fft: &'a FloatRealFft,

    // samples at 200hz, directly stored
    samples_list: Vec<f32, 40>,
    low_pass_filter: DirectForm2Transposed<f32>,
}

impl<'a> SGChannelProcessor<'a> {
    fn new(fft: &'a FloatRealFft) -> Self {
        let cutoff_freq = 100.hz();
        let sampling_freq = ((SAMPLES_PER_READ * READS_PER_SECOND) as u32).hz();

        let coeffs = Coefficients::<f32>::from_params(
            Type::LowPass,
            sampling_freq,
            cutoff_freq,
            Q_BUTTERWORTH_F32,
        )
        .unwrap();
        let low_pass_filter = DirectForm2Transposed::<f32>::new(coeffs);
        Self {
            fft_samples_sum: [0.0; FFT_SIZE],
            fft_samples_sum_count: 0,
            fft_i: 0,
            fft,

            samples_list: Vec::new(),
            low_pass_filter,
        }
    }

    fn process<'b, T: RawSGReadingsTrait>(&mut self, mut raw_readings_iter: T::Iter<'b>) {
        let mut skip_fft = false;
        for i in 0..SAMPLES_PER_READ {
            let raw_reading = raw_readings_iter.next().unwrap();
            let low_passed_reading = self.low_pass_filter.run(raw_reading);
            if i == SAMPLES_PER_READ - 1 {
                self.samples_list.push(low_passed_reading).unwrap();
            }

            if !skip_fft {
                self.fft_samples_sum[self.fft_i] += raw_reading;
                self.fft_i += 1;
                if self.fft_i == FFT_SIZE {
                    self.fft_i = 0;
                    self.fft_samples_sum_count += 1;
                    skip_fft = true;
                }
            }
        }
    }

    fn process_fft(
        &mut self,
        fft_out_buffer: &mut [f32],
        processed_reading: &mut ProcessedSGReading,
    ) {
        defmt::debug_assert!(fft_out_buffer.len() == FFT_SIZE);

        self.fft.run(&mut self.fft_samples_sum, fft_out_buffer);
        // since the real-valued coefficient at the Nyquist frequency is packed into the
        // imaginary part of the DC bin, it must be cleared before computing the amplitudes
        fft_out_buffer[1] = 0.0;

        // let mut max_amplitudes = [(0usize, half::f16::from_f32_const(0.0)); 5];

        for i in 0..200 {
            let amplitude = half::f16::from_f32(unsafe {
                intrinsics::sqrtf32(
                    fft_out_buffer[i * 2] * fft_out_buffer[i * 2]
                        + fft_out_buffer[i * 2 + 1] * fft_out_buffer[i * 2 + 1],
                ) / (FFT_SIZE as f32)
                    / self.fft_samples_sum_count as f32
            });

            // if self.sg_i == 1 {
            //     let frequency = i * 20 + 10;
            //     for j in 0..max_amplitudes.len() {
            //         if amplitude > max_amplitudes[j].1 {
            //             (&mut max_amplitudes[j..]).rotate_right(1);
            //             max_amplitudes[j] = (frequency, amplitude);
            //             break;
            //         }
            //     }
            // }

            // let amplitude_bytes = amplitude.to_le_bytes();
            // processed_reading.amplitudes[i * 2] = amplitude_bytes[0];
            // processed_reading.amplitudes[i * 2 + 1] = amplitude_bytes[1];
        }

        // if self.sg_i == 1 {
        //     for i in 0..max_amplitudes.len() {
        //         let (frequency, amplitude) = max_amplitudes[i];
        //         log_info!("SG{}: {}Hz {}", self.sg_i, frequency, amplitude.to_f32());
        //     }
        //     log_info!("=====================");
        // }

        self.fft_samples_sum = [0.0; FFT_SIZE];
        self.fft_samples_sum_count = 0;
        self.fft_i = 0;

        for i in 0..self.samples_list.len() {
            let sample = half::f16::from_f32(self.samples_list[i]);
            let sample_bytes = sample.to_le_bytes();
            processed_reading.samples[i * 2] = sample_bytes[0];
            processed_reading.samples[i * 2 + 1] = sample_bytes[1];
        }
        self.samples_list.clear();
    }
}

pub async fn sg_low_prio_main<T: RawSGReadingsTrait>(states: &SGGlobalStates<impl RawMutex, T>) {
    let realtime_pub = states.realtime_sample_pubsub.publisher().unwrap();
    let fft = FloatRealFft::new(FFT_SIZE as u16).unwrap();
    let mut fft_out_buffer = [0f32; FFT_SIZE];

    let mut sg_processor_list: [SGChannelProcessor; 4] =
        array::from_fn(|_| SGChannelProcessor::new(&fft));

    let mut raw_readings_receiver = states.raw_readings_channel.receiver();
    let mut processed_readings_sender = states.processed_readings_channel.sender();
    loop {
        let mut start_time: Option<f64> = None;

        for i in 0..(BATCH_SIZE / SAMPLES_PER_READ * 4) {
            let raw_readings = raw_readings_receiver.receive().await;

            if i == 0 {
                start_time = Some(raw_readings.get_start_time());
            }

            // takes 1.5ms
            for sg_i in 0..4 {
                let processor = &mut sg_processor_list[sg_i];
                let sg_readings_iter = raw_readings.get_sg_readings_iter(sg_i);
                processor.process::<T>(sg_readings_iter);
            }

            realtime_pub.publish_immediate([
                *sg_processor_list[0].samples_list.last().unwrap(),
                *sg_processor_list[1].samples_list.last().unwrap(),
                *sg_processor_list[2].samples_list.last().unwrap(),
                *sg_processor_list[3].samples_list.last().unwrap(),
            ]);
            // log_info!(
            //     "SG readings: {}V {}V {}V {}V",
            //     sg_processor_list[0].samples_list.last().unwrap(),
            //     sg_processor_list[1].samples_list.last().unwrap(),
            //     sg_processor_list[2].samples_list.last().unwrap(),
            //     sg_processor_list[3].samples_list.last().unwrap(),
            // );
        }

        // takes 11ms
        for sg_i in 0..4 {
            let mut processed_readings = processed_readings_sender.start_send().await;
            processed_readings.start_time = start_time.unwrap();
            processed_readings.sg_i = sg_i as u8;

            let processor = &mut sg_processor_list[sg_i];
            processor.process_fft(&mut fft_out_buffer, &mut processed_readings);
        }
    }
}
