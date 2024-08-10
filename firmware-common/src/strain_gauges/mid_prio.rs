use core::ops::DerefMut;

use embassy_sync::blocking_mutex::raw::RawMutex;
use futures::join;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::{Crc, Flash, StatFlash, VLFS};

use crate::common::can_bus::node_types::STRAIN_GAUGES_NODE_TYPE;
use crate::common::delta_logger::prelude::{RingDeltaLoggerConfig, RingFileWriter};
use crate::common::file_types::SG_READINGS;
use crate::create_serialized_enum;
use crate::driver::can_bus::can_node_id_from_serial_number;
use crate::driver::clock::{Clock, VLFSTimerWrapper};
use crate::driver::delay::Delay;
use crate::driver::sg_adc::SGAdcController;
use crate::driver::{can_bus::SplitableCanBus, indicator::Indicator};
use crate::strain_gauges::global_states::SGLEDState;

use super::global_states::SGGlobalStates;
use super::{ArchivedProcessedSGReading, ProcessedSGReading};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct UnixTimestampLog {
    pub boot_timestamp: f64,
    pub unix_timestamp: f64,
}

create_serialized_enum!(
    SGReadingLogger,
    SGReadingLoggerReader,
    SGReadingLog,
    (0, ProcessedSGReading),
    (1, UnixTimestampLog)
);

pub async fn mid_prio_main(
    state: &SGGlobalStates<impl RawMutex>,
    device_serial_number: &[u8; 12],
    mut indicator: impl Indicator,
    mut sg_adc_controller: impl SGAdcController,
    flash: impl Flash,
    crc: impl Crc,
    mut can: impl SplitableCanBus,
    clock: impl Clock,
    delay: impl Delay,
) {
    log_info!("Initializing VLFS");
    let stat_flash = StatFlash::new();
    let mut flash = stat_flash.get_flash(flash, VLFSTimerWrapper(clock.clone()));
    flash.reset().await.unwrap();
    let mut fs = VLFS::new(flash, crc);
    fs.init().await.unwrap();

    let sg_reading_ring_writer = RingFileWriter::new(
        &fs,
        RingDeltaLoggerConfig {
            file_type: SG_READINGS,
            seconds_per_segment: 5 * 60,
            first_segment_seconds: 5 * 60,
            segments_per_ring: 6,
        },
        delay.clone(),
        clock.clone(),
    )
    .await
    .unwrap();
    let sg_reading_ring_writer_fut = sg_reading_ring_writer.run();
    let mut sg_reading_logger = SGReadingLogger::new();

    log_info!("Initializing CAN Bus");
    can.reset().await.unwrap();
    can.configure_self_node(
        STRAIN_GAUGES_NODE_TYPE,
        can_node_id_from_serial_number(device_serial_number),
    );
    let (mut can_tx, mut can_rx) = can.split();
    // TODO can bus

    let store_sg_fut = async {
        let processed_readings_receiver = state.processed_readings_channel.receiver();
        sg_adc_controller.set_enable(true).await; // TODO only do this in dev mode
        loop {
            let readings = processed_readings_receiver.receive().await;
            let start = clock.now_ms();

            let (_, mut writer) = sg_reading_ring_writer.get_writer().await;
            for reading in readings.iter() {
                let result = sg_reading_logger
                    .write(
                        writer.deref_mut().as_mut().unwrap(),
                        &SGReadingLog::ProcessedSGReading(reading.clone()),
                    )
                    .await;

                if let Err(e) = result {
                    log_warn!("Write failed, {:?}", e);
                }
            }
            drop(writer);

            let end = clock.now_ms();
            log_info!(
                "Wrote readings in {}ms, free space: {}KiB",
                end - start,
                fs.free().await as f32 / 1024.0
            );
        }
    };

    let led_fut = async {
        loop {
            let led_state = state.led_state.lock(|s| s.borrow().clone());
            match led_state {
                SGLEDState {
                    can_bus_error: true,
                    ..
                } => loop {
                    indicator.set_enable(true).await;
                    delay.delay_ms(100.0).await;
                    indicator.set_enable(false).await;
                    delay.delay_ms(100.0).await;
                },
                SGLEDState {
                    can_bus_error: false,
                    ..
                } => {
                    indicator.set_enable(true).await;
                    delay.delay_ms(100.0).await;
                }
            }
        }
    };

    join!(sg_reading_ring_writer_fut, store_sg_fut, led_fut);
}
