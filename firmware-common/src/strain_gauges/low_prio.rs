use core::{array, intrinsics};

use biquad::*;
use cmsis_dsp::transform::FloatRealFft;
use embassy_sync::blocking_mutex::raw::RawMutex;
use heapless::Vec;

use super::{global_states::SGGlobalStates, ProcessedSGReading, READS_PER_SECOND, SAMPLES_PER_READ};

const FFT_SIZE: usize = 2048;

struct SGChannelProcessor<'a> {
    // samples used for fft
    fft_samples_sum: [f32; FFT_SIZE],
    fft_samples_sum_count: usize,
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
            fft,

            samples_list: Vec::new(),
            low_pass_filter,
        }
    }

    fn process(&mut self, raw_readings: &[f32]) {
        defmt::debug_assert!(raw_readings.len() == SAMPLES_PER_READ);

        for j in 0..10 {
            for _ in 0..(SAMPLES_PER_READ / 10 - 1) {
                self.low_pass_filter.run(raw_readings[j]);
            }
            self.samples_list
                .push(self.low_pass_filter.run(raw_readings[j]))
                .unwrap();
        }

        for i in 0..FFT_SIZE {
            self.fft_samples_sum[i] += raw_readings[i];
        }
        self.fft_samples_sum_count += 1;
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
        for i in 0..750 {
            let amplitude = half::f16::from_f32(unsafe {
                intrinsics::sqrtf32(
                    fft_out_buffer[i * 2] * fft_out_buffer[i * 2]
                        + fft_out_buffer[i * 2 + 1] * fft_out_buffer[i * 2 + 1],
                ) / 750f32
                    / self.fft_samples_sum_count as f32
            });
            let amplitude_bytes = amplitude.to_le_bytes();
            processed_reading.amplitudes[i * 2] = amplitude_bytes[0];
            processed_reading.amplitudes[i * 2 + 1] = amplitude_bytes[1];
        }

        self.fft_samples_sum = [0.0; FFT_SIZE];
        self.fft_samples_sum_count = 0;

        for i in 0..self.samples_list.len() {
            let sample = half::f16::from_f32(self.samples_list[i]);
            let sample_bytes = sample.to_le_bytes();
            processed_reading.samples[i * 2] = sample_bytes[0];
            processed_reading.samples[i * 2 + 1] = sample_bytes[1];
        }
        self.samples_list.clear();
    }
}

pub async fn low_prio_main(state: &SGGlobalStates<impl RawMutex>) {
    let raw_readings_receiver = state.raw_readings_channel.receiver();
    let mut processed_readings_sender = state.processed_readings_channel.sender();

    let fft = FloatRealFft::new(FFT_SIZE as u16).unwrap();
    let mut fft_out_buffer = [0f32; FFT_SIZE];

    let mut sg_processor_list: [SGChannelProcessor; 4] =
        array::from_fn(|_| SGChannelProcessor::new(&fft));

    loop {
        let mut start_time: Option<f64> = None;

        for i in 0..4 {
            let mut raw_readings = raw_readings_receiver.receive().await;

            if i == 0 {
                start_time = Some(raw_readings.start_time);
            }

            // takes 1.5ms
            for sg_i in 0..4 {
                let processor = &mut sg_processor_list[sg_i];
                processor.process(&mut raw_readings.sg_readings[sg_i]);
            }

            // log_info!(
            //     "SG readings: {}V {}V {}V {}V",
            //     sg_processor_list[0].samples_list.last().unwrap(),
            //     sg_processor_list[1].samples_list.last().unwrap(),
            //     sg_processor_list[2].samples_list.last().unwrap(),
            //     sg_processor_list[3].samples_list.last().unwrap(),
            // );
        }

        let mut processed_readings = processed_readings_sender.start_send().await;
        // takes 11ms
        for sg_i in 0..4 {
            let processor = &mut sg_processor_list[sg_i];
            processed_readings[sg_i].start_time = start_time.unwrap();
            processor.process_fft(&mut fft_out_buffer, &mut processed_readings[sg_i])
        }
        drop(processed_readings)
    }
}
