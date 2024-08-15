use rkyv::{Archive, Deserialize, Serialize};

pub mod global_states;
pub mod high_prio;
pub mod low_prio;
pub mod mid_prio;

// Game plan:
// for each channel:
//     collect 213 samples (5ms) each adc read (200 times per second)
//     batch every 10 reads together, total 2130 samples (50ms)
//     1. Steps to get vibration frequencies:
//         a. truncate to 2048 samples, and do fft on it, gives you frequency from 0hz to 20.48kHz in 20Hz increments
//         b. throw away frequencies above 4kHz, since thats the cutoff frequency of the rc filter on the pcb (the rc filter should be 15kHz cutoff by itself, but that calculation does not take into account the output impedance of the opamp)
//         c. average the frequencies every 4 fft (200 values, each 2 bytes, per 200ms)
//     2. run low pass filter over the samples to only keep < 100Hz, and down sample to 200 samples/s, (200 values, each 2 bytes, per 1s)
//     200 * 5 * 2 + 200 * 2 = 2400 bytes per channel per second
// total data rate (before any compression): 2400 * 4 = 9.6KiB/s
// 64MiB flash can store 64MiB / 9.6KiB/s = 113 minutes of data

pub const BATCH_SIZE: usize = 2130; // one batch every 50ms
pub const SAMPLES_PER_READ: usize = BATCH_SIZE / 10;
pub const READS_PER_SECOND: usize = 200;

// one per 200ms per channel
#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ProcessedSGReading {
    pub start_time: f64, // boot time in ms
    pub sg_i: u8,
    pub amplitudes: [u8; 400], // [half::f16; 200]
    pub samples: [u8; 80],      // [half::f16; 40]
}

impl Default for ProcessedSGReading {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            sg_i: 0,
            amplitudes: [0u8; 400],
            samples: [0u8; 80],
        }
    }
}
