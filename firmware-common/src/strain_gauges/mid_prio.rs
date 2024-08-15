use core::cell::RefCell;
use core::ops::DerefMut;

use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use futures::join;
use vlfs::{Crc, Flash, VLFS};

use crate::common::can_bus::id::CanBusExtendedId;
use crate::common::can_bus::message::CanBusMessage as _;
use crate::common::can_bus::messages::{
    AvionicsStatusMessage, FlightEvent, FlightEventMessage, HealthMessage, HealthState,
    ResetMessage, UnixTimeMessage,
};
use crate::common::can_bus::node_types::STRAIN_GAUGES_NODE_TYPE;
use crate::common::console::sg_rpc::run_rpc_server;
use crate::common::delta_logger::buffered_logger::BufferedLoggerState;
use crate::common::delta_logger::delta_logger::{ArchivedUnixTimestampLog, UnixTimestampLog};
use crate::common::delta_logger::prelude::RingFileWriter;
use crate::common::delta_logger::ring_delta_logger::{RingDeltaLoggerConfig, RingDeltaLoggerState};
use crate::common::file_types::{SG_BATTERY_LOGGER, SG_READINGS};
use crate::common::ticker::Ticker;
use crate::driver::adc::{ADCData, Volt, ADC};
use crate::driver::can_bus::{
    can_node_id_from_serial_number, CanBusRX, CanBusRawMessage as _, CanBusTX,
};
use crate::driver::clock::Clock;
use crate::driver::delay::Delay;
use crate::driver::sg_adc::{RawSGReadingsTrait, SGAdcController};
use crate::driver::sys_reset::SysReset;
use crate::driver::usb::SplitableUSB;
use crate::driver::{can_bus::SplitableCanBus, indicator::Indicator};
use crate::{create_serialized_enum, fixed_point_factory, try_or_warn};

use super::global_states::SGGlobalStates;
use super::{ArchivedProcessedSGReading, ProcessedSGReading};

create_serialized_enum!(
    SGReadingLogger,
    SGReadingLoggerReader,
    SGReadingLog,
    (0, ProcessedSGReading),
    (1, UnixTimestampLog)
);

fixed_point_factory!(BatteryFF, f64, 4999.0, 5010.0, 0.5);

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
    mut usb: impl SplitableUSB,
    mut battery_adc: impl ADC<Volt>,
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

    log_info!("Initializing CAN Bus");
    let (mut can_tx, mut can_rx) = can.split();
    can_tx.configure_self_node(
        STRAIN_GAUGES_NODE_TYPE,
        can_node_id_from_serial_number(device_serial_number),
    );

    let usb_connected = {
        log_info!("Waiting for USB connection");
        let timeout_fut = delay.delay_ms(500.0);
        let usb_wait_connection_fut = usb.wait_connection();
        match select(timeout_fut, usb_wait_connection_fut).await {
            Either::First(_) => {
                log_info!("USB not connected");
                false
            }
            Either::Second(_) => {
                log_info!("USB connected");
                true
            }
        }
    };

    log_info!("Initializing RPC Server");
    let usb_console_fut = async {
        loop {
            usb.wait_connection().await;
            run_rpc_server(&mut usb, &fs, &sys_reset, device_serial_number).await;
        }
    };

    let can_rx_fut = async {
        let delay = delay.clone();
        let mut armed = false;
        let mut landed = false;
        loop {
            match can_rx.receive().await {
                Ok(message) => {
                    states.error_states.lock(|s| {
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
                    states.error_states.lock(|s| {
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
            let error_states = states.error_states.lock(|s| s.borrow().clone());
            let health_message = HealthMessage {
                state: if error_states.can_bus_error || error_states.sg_adc_error {
                    HealthState::UnHealthy
                } else {
                    HealthState::Healthy
                },
            };
            can_tx.send(&health_message, 3).await.ok();
            ticker.next().await;
        }
    };

    let led_fut = async {
        loop {
            let error_states = states.error_states.lock(|s| s.borrow().clone());
            if error_states.usb_connected {
                indicator.set_enable(true).await;
                delay.delay_ms(50.0).await;
                indicator.set_enable(false).await;
                delay.delay_ms(100.0).await;
                indicator.set_enable(true).await;
                delay.delay_ms(50.0).await;
                indicator.set_enable(false).await;
                delay.delay_ms(800.0).await;
            } else if error_states.can_bus_error || error_states.sg_adc_error {
                indicator.set_enable(true).await;
                delay.delay_ms(100.0).await;
                indicator.set_enable(false).await;
                delay.delay_ms(100.0).await;
            } else {
                indicator.set_enable(true).await;
                delay.delay_ms(50.0).await;
                indicator.set_enable(false).await;
                delay.delay_ms(950.0).await;
            }
        }
    };

    let store_sg_fut = async {
        if usb_connected {
            log_info!("USB connected on boot, stopping");
            states.error_states.lock(|led_state| {
                led_state.borrow_mut().usb_connected = true;
            });
            #[cfg(not(debug_assertions))]
            return;
        }

        log_info!("Creating SG logger");
        let sg_reading_ring_writer = RingFileWriter::new(
            &fs,
            RingDeltaLoggerConfig {
                file_type: SG_READINGS,
                seconds_per_segment: 120,
                first_segment_seconds: 120,
                segments_per_ring: 45, // 90 mins of data
            },
            delay.clone(),
            clock.clone(),
        )
        .await
        .unwrap();
        let sg_reading_ring_writer_fut = sg_reading_ring_writer.run();
        let mut sg_reading_logger = SGReadingLogger::new();

        log_info!("Creating battery logger");
        let battery_logger_state =
            RingDeltaLoggerState::<ADCData<Volt>, _, _, BatteryFF, _, _>::new(
                &fs,
                delay.clone(),
                clock.clone(),
                RingDeltaLoggerConfig {
                    file_type: SG_BATTERY_LOGGER,
                    seconds_per_segment: 1800,
                    first_segment_seconds: 60,
                    segments_per_ring: 40, // 20 hours
                },
            )
            .await
            .unwrap();
        let (battery_logger, mut battery_logger_runner) = battery_logger_state.get_logger_runner();
        let buffered_battery_logger_state = BufferedLoggerState::<_, _, _, 10>::new(battery_logger);
        let (buffered_battery_logger, mut buffered_batter_logger_runner) =
            buffered_battery_logger_state.get_logger_runner();

        let processed_readings_receiver_fut = async {
            let clock = clock.clone();
            let mut processed_readings_receiver = states.processed_readings_channel.receiver();
            // #[cfg(debug_assertions)]
            sg_adc_controller.lock().await.set_enable(true).await;

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
                    try_or_warn!(
                        sg_reading_logger
                            .write(
                                writer.deref_mut().as_mut().unwrap(),
                                &SGReadingLog::UnixTimestampLog(unix_time_log),
                            )
                            .await
                    );
                }

                try_or_warn!(
                    sg_reading_logger
                        .write(
                            writer.deref_mut().as_mut().unwrap(),
                            &SGReadingLog::ProcessedSGReading(reading.clone()),
                        )
                        .await
                );
                drop(writer);

                let end = clock.now_ms();
                log_info!(
                    "Wrote sg {} readings in {}ms, free space: {}KiB",
                    reading.sg_i,
                    end - start,
                    fs.free().await as f32 / 1024.0
                );
            }
        };

        let store_battery_fut = async {
            loop {
                let battery_reading = battery_adc.read().await.unwrap();
                buffered_battery_logger.ref_log(battery_reading);
            }
        };

        let store_battery_unix_time_fut = async {
            let mut ticker = Ticker::every(clock.clone(), delay.clone(), 5.0 * 60.0 * 1000.0);
            let mut last_unix_time_log: Option<UnixTimestampLog> = None;

            loop {
                ticker.next().await;
                if let Some((unix_time_log, _)) =
                    unix_timestamp_log_mutex.lock(|m| m.borrow().clone())
                {
                    if let Some(last_unix_time_log) = &mut last_unix_time_log {
                        if unix_time_log != *last_unix_time_log {
                            buffered_battery_logger.ref_log_unix_time(unix_time_log.clone());
                            *last_unix_time_log = unix_time_log;
                        }
                    } else {
                        buffered_battery_logger.ref_log_unix_time(unix_time_log.clone());
                        last_unix_time_log.replace(unix_time_log);
                    }
                }
            }
        };

        #[allow(unused_must_use)]
        {
            join!(
                sg_reading_ring_writer_fut,
                processed_readings_receiver_fut,
                store_battery_fut,
                store_battery_unix_time_fut,
                battery_logger_runner.run(),
                buffered_batter_logger_runner.run()
            );
        }
    };

    join!(
        usb_console_fut,
        store_sg_fut,
        led_fut,
        can_rx_fut,
        can_tx_fut
    );
}
