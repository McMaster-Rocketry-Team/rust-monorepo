use core::ops::DerefMut;
use std::cell::RefCell;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use futures::join;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::{Crc, Flash, VLFS};

use crate::common::can_bus::id::CanBusExtendedId;
use crate::common::can_bus::message::CanBusMessage as _;
use crate::common::can_bus::messages::{
    AvionicsStatusMessage, FlightEvent, FlightEventMessage, HealthMessage, HealthState,
    ResetMessage, UnixTimeMessage,
};
use crate::common::can_bus::node_types::STRAIN_GAUGES_NODE_TYPE;
use crate::common::delta_logger::prelude::{RingDeltaLoggerConfig, RingFileWriter};
use crate::common::file_types::SG_READINGS;
use crate::common::ticker::Ticker;
use crate::create_serialized_enum;
use crate::driver::can_bus::{
    can_node_id_from_serial_number, CanBusRX, CanBusRawMessage as _, CanBusTX,
};
use crate::driver::clock::Clock;
use crate::driver::delay::Delay;
use crate::driver::sg_adc::{RawSGReadingsTrait, SGAdcController};
use crate::driver::sys_reset::SysReset;
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

pub async fn sg_mid_prio_main(
    states: &SGGlobalStates<impl RawMutex, impl RawSGReadingsTrait>,
    device_serial_number: &[u8; 12],
    mut indicator: impl Indicator,
    sg_adc_controller: impl SGAdcController,
    mut flash: impl Flash,
    crc: impl Crc,
    mut can: impl SplitableCanBus,
    clock: impl Clock,
    delay: impl Delay,
    sys_reset: impl SysReset,
) {
    let sg_adc_controller = Mutex::<NoopRawMutex, _>::new(sg_adc_controller);
    let unix_timestamp_log_mutex = BlockingMutex::<
        NoopRawMutex,
        RefCell<Option<(UnixTimestampLog, bool)>>,
    >::new(RefCell::new(None));

    log_info!("Initializing VLFS");
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

    let can_rx_fut = async {
        let delay = delay.clone();
        let mut armed = false;
        let mut landed = false;
        loop {
            match can_rx.receive().await {
                Ok(message) => {
                    states.led_state.lock(|s| {
                        s.borrow_mut().can_bus_error = false;
                    });

                    if message.rtr() {
                        continue;
                    }
                    let id = CanBusExtendedId::from_raw(message.id());

                    if id.message_type == AvionicsStatusMessage::message_type() {
                        // if armed -> going to launch soon, start recording adc
                        let message =
                            AvionicsStatusMessage::from_data(message.data().try_into().unwrap());
                        armed = message.armed;
                    } else if id.message_type == FlightEventMessage::message_type() {
                        // if landed -> stop recording adc
                        let message =
                            FlightEventMessage::from_data(message.data().try_into().unwrap());
                        landed = message.event == FlightEvent::Landed;
                    } else if id.message_type == UnixTimeMessage::message_type() {
                        // sync time
                        let boot_timestamp = message.timestamp();
                        let message =
                            UnixTimeMessage::from_data(message.data().try_into().unwrap());
                        let unix_timestamp: u64 = message.timestamp.into();
                        unix_timestamp_log_mutex.lock(|m| {
                            m.borrow_mut().replace((
                                UnixTimestampLog {
                                    boot_timestamp,
                                    unix_timestamp: unix_timestamp as f64,
                                },
                                true,
                            ));
                        });
                    } else if id.message_type == ResetMessage::message_type() {
                        sys_reset.reset();
                    }

                    sg_adc_controller
                        .lock()
                        .await
                        .set_enable(armed && !landed)
                        .await;
                }
                Err(_) => {
                    states.led_state.lock(|s| {
                        s.borrow_mut().can_bus_error = true;
                    });
                    delay.delay_ms(150.0).await;
                }
            }
        }
    };

    let can_tx_fut = async {
        let mut ticker = Ticker::every(clock.clone(), delay.clone(), 1000.0);
        loop {
            let health_message = HealthMessage {
                state: HealthState::Healthy,
            };
            can_tx.send(&health_message, 3).await.ok();
            ticker.next().await;
        }
    };

    let store_sg_fut = async {
        let processed_readings_receiver = states.processed_readings_channel.receiver();
        sg_adc_controller.lock().await.set_enable(true).await; // TODO only do this in dev mode

        loop {
            let reading = processed_readings_receiver.receive().await;
            let start = clock.now_ms();

            let (new_file, mut writer) = sg_reading_ring_writer.get_writer().await;
            if let Some(unix_time_log) = unix_timestamp_log_mutex.lock(|m| {
                let mut m = m.borrow_mut();

                if let Some((unix_time_log, updated)) = m.as_mut() {
                    if new_file || *updated {
                        *updated = false;
                        return Some(unix_time_log.clone());
                    }
                }
                return None;
            }) {
                sg_reading_logger
                    .write(
                        writer.deref_mut().as_mut().unwrap(),
                        &SGReadingLog::UnixTimestampLog(unix_time_log),
                    )
                    .await
                    .ok();
            }

            let result = sg_reading_logger
                .write(
                    writer.deref_mut().as_mut().unwrap(),
                    &SGReadingLog::ProcessedSGReading(reading.clone()),
                )
                .await;

            if let Err(e) = result {
                log_warn!("Write failed, {:?}", e);
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
            let led_state = states.led_state.lock(|s| s.borrow().clone());
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
                    delay.delay_ms(50.0).await;
                    indicator.set_enable(false).await;
                    delay.delay_ms(950.0).await;
                }
            }
        }
    };

    join!(
        sg_reading_ring_writer_fut,
        store_sg_fut,
        led_fut,
        can_rx_fut,
        can_tx_fut
    );
}
