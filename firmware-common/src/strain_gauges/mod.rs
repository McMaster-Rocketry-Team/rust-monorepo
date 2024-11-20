use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use global_states::SGGlobalStates;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::{Crc, Flash};

use crate::{
    driver::{
        adc::{Volt, ADC},
        can_bus::SplitableCanBus,
        clock::Clock,
        delay::Delay,
        indicator::Indicator,
        sg_adc::{RawSGReadingsTrait, SGAdc, SGAdcController},
        spawner::SendSpawner,
        sys_reset::SysReset,
        usb::SplitableUSB,
    },
    sg_high_prio_main, sg_low_prio_main, sg_mid_prio_main,
};

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
    // pub amplitudes: [u8; 400], // [half::f16; 200]
    pub samples: [u8; 80], // [half::f16; 40]
}

impl Default for ProcessedSGReading {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            sg_i: 0,
            // amplitudes: [0u8; 400],
            samples: [0u8; 80],
        }
    }
}

pub fn sg_main<T, SG, I, SC, F, C, N, CL, DL, R, U, A>(
    high_prio_spawner: &'static impl SendSpawner,
    mid_prio_spawner: &'static impl SendSpawner,
    low_prio_spawner: &'static impl SendSpawner,
    global_states: &'static SGGlobalStates<CriticalSectionRawMutex, T>,
    sg_adc: impl (FnOnce() -> SG) + Send,

    device_serial_number: [u8; 12],
    indicator: impl (FnOnce() -> I) + Send,
    sg_adc_controller: impl (FnOnce() -> SC) + Send,
    flash: impl (FnOnce() -> F) + Send,
    crc: impl (FnOnce() -> C) + Send,
    can: impl (FnOnce() -> N) + Send,
    clock: impl (FnOnce() -> CL) + Send,
    delay: impl (FnOnce() -> DL) + Send,
    sys_reset: impl (FnOnce() -> R) + Send,
    usb: impl (FnOnce() -> U) + Send,
    battery_adc: impl (FnOnce() -> A) + Send,
) where
    T: RawSGReadingsTrait + Send,
    SG: SGAdc<T> + 'static,
    I: Indicator + 'static,
    SC: SGAdcController + 'static,
    F: Flash + 'static,
    C: Crc + 'static,
    N: SplitableCanBus + 'static,
    CL: Clock + 'static,
    DL: Delay + 'static,
    R: SysReset + 'static,
    U: SplitableUSB + 'static,
    A: ADC<Volt> + 'static,
{
    high_prio_spawner.spawn(move || sg_high_prio_main(global_states, sg_adc()));
    mid_prio_spawner.spawn(move || {
        sg_mid_prio_main(
            global_states,
            device_serial_number,
            indicator(),
            sg_adc_controller(),
            flash(),
            crc(),
            can(),
            clock(),
            delay(),
            sys_reset(),
            usb(),
            battery_adc(),
        )
    });
    low_prio_spawner.spawn(move || sg_low_prio_main(global_states));
}
