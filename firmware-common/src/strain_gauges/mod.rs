use rkyv::{Archive, Deserialize, Serialize};

pub mod global_states;
pub mod high_prio;
pub mod mid_prio;
pub mod low_prio;


// Game plan:
// for each channel:
//     collect 2130 samples (50ms) each adc read (20 times per second)
//     1. Steps to get vibration frequencies:
//         a. truncate to 2048 samples, and do fft on it, gives you frequency from 0hz to 20.48kHz in 20Hz increments
//         b. throw away frequencies above 15kHz, since thats the cutoff frequency of the rc filter on the pcb
//         c. average the frequencies every 4 fft (750 values, each 2 bytes, per 200ms)
//     2. run low pass filter over the samples to only keep < 100Hz, and down sample to 200 samples/s, (200 values, each 2 bytes, per 1s)
//     750 * 5 * 2 + 200 * 2 = 7900 bytes per channel per second
// total data rate (before any compression): 4150 * 4 = 31.6KiB/s
// 64MiB flash can store 64MiB / 31.6KiB/s = 34 minutes of data

pub const SAMPLES_PER_READ: usize = 2130;
pub const READS_PER_SECOND: usize = 20;


// one per 200ms per channel
#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct ProcessedSGReading {
    pub start_time: f64, // boot time in ms
    pub sg_i: u8,
    pub amplitudes: [u8; 1500], // [half::f16; 750]
    pub samples: [u8; 80],      // [half::f16; 40]
}

impl ProcessedSGReading {
    pub const fn new_const(sg_i: u8) -> Self {
        Self {
            start_time: 0.0,
            sg_i,
            amplitudes: [0u8; 1500],
            samples: [0u8; 80],
        }
    }
}
