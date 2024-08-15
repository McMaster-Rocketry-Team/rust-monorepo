use core::cell::RefCell;

use crate::{
    avionics::{arming_state::ArmingStateManager, flight_profile::PyroSelection},
    claim_devices,
    common::{
        can_bus::{messages as can_messages, node_types::VOID_LAKE_NODE_TYPE}, delta_logger::{buffered_logger::BufferedLoggerState, delta_logger::DeltaLogger, prelude::DeltaLoggerTrait}, device_config::{DeviceConfig, DeviceModeConfig}, file_types::{GROUND_TEST_BARO_FILE_TYPE, GROUND_TEST_LOG_FILE_TYPE}, ticker::Ticker, vl_device_manager::prelude::*, vlp::{
            packet::{GroundTestDeployPacket, VLPDownlinkPacket, VLPUplinkPacket},
            telemetry_packet::TelemetryPacketBuilder,
            uplink_client::VLPUplinkClient,
        }
    },
    create_serialized_enum,
    driver::{barometer::BaroData, can_bus::{can_node_id_from_serial_number, CanBusTX}, indicator::Indicator},
    fixed_point_factory, pyro, try_or_warn, vl_device_manager_type,
};
use embassy_sync::{blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex}, mutex::Mutex};
use futures::join;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct FireEvent {
    pub timestamp: f64, // ms
}

create_serialized_enum!(
    GroundTestLogger,
    GroundTestLoggerReader,
    GroundTestLog,
    (0, FireEvent)
);

#[inline(never)]
pub async fn ground_test_avionics(
    device_manager: vl_device_manager_type!(),
    services: system_services_type!(),
    config: &DeviceConfig,
    device_serial_number: &[u8; 12],
) -> ! {
    let (drogue_pyro, main_pyro) = if let DeviceModeConfig::GroundTestAvionics {
        drogue_pyro,
        main_pyro,
    } = config.mode
    {
        (drogue_pyro, main_pyro)
    } else {
        log_unreachable!()
    };

    claim_devices!(device_manager, lora, barometer, arming_switch, indicators, can_bus);

    log_info!("Creating logger");
    let mut log_file_writer = services
        .fs
        .create_file_and_open_for_write(GROUND_TEST_LOG_FILE_TYPE)
        .await
        .unwrap();
    let mut logger = GroundTestLogger::new();

    log_info!("Creating baro logger");
    fixed_point_factory!(BaroFF, f64, 4.9, 7.0, 0.05);
    let baro_log_file_writer = services
        .fs
        .create_file_and_open_for_write(GROUND_TEST_BARO_FILE_TYPE)
        .await
        .unwrap();
    let baro_logger = DeltaLogger::<BaroData, _, BaroFF>::new(baro_log_file_writer);
    let buffered_baro_logger_state = BufferedLoggerState::<_, _, _, 100>::new(baro_logger);
    let (mut buffered_baro_logger, mut buffered_baro_logger_runner) =
        buffered_baro_logger_state.get_logger_runner();

    log_info!("resetting barometer");
    barometer.reset().await.unwrap();

    let arming_state = ArmingStateManager::<NoopRawMutex>::new();
    arming_state.set_software_armed(true);
    let arming_state_debounce_fut = arming_state.run_debounce(services.delay.clone());

    let indicator_fut = indicators.run([], [50, 2000], []);

    let mut can_bus = can_bus.take().unwrap();
    let (mut can_tx, _) = can_bus.split();
    can_tx.configure_self_node(
        VOID_LAKE_NODE_TYPE,
        can_node_id_from_serial_number(device_serial_number),
    );

    let can_tx = Mutex::<NoopRawMutex, _>::new(can_tx);

    let can_tx_avionics_status_fut = async {
        let mut ticker = Ticker::every(services.clock(), services.delay(), 2000.0);
        loop {
            let message = can_messages::AvionicsStatusMessage {
                low_power: false,
                armed: arming_state.is_armed(),
            };
            let mut can_tx = can_tx.lock().await;
            can_tx.send(&message, 3).await.ok();
            drop(can_tx);

            ticker.next().await;
        }
    };

    let can_tx_unix_time_fut = async {
        let mut unix_clock_sub = services.unix_clock.subscribe_unix_clock_update();
        loop {
            let unix_timestamp = unix_clock_sub.next_message_pure().await;
            let mut can_tx = can_tx.lock().await;
            let message = can_messages::UnixTimeMessage {
                timestamp: (unix_timestamp as u64).into(),
            };
            can_tx.send(&message, 2).await.ok();
            drop(can_tx);
        }
    };

    let telemetry_packet_builder = TelemetryPacketBuilder::new(services.unix_clock());
    let vlp = VLPUplinkClient::new();
    let vlp_tx_fut = async {
        let mut update_ticker = Ticker::every(services.clock(), services.delay(), 1000.0);
        loop {
            update_ticker.next().await;

            let free = services.fs.free().await;
            telemetry_packet_builder.update(|b| {
                b.disk_free_space = free;
            });
            let packet = telemetry_packet_builder.create_packet();
            vlp.send(VLPDownlinkPacket::TelemetryPacket(packet));
        }
    };
    let vlp_rx_fut = async {
        let (packet, _) = vlp.wait_receive().await;
        log_info!("Received packet: {:?}", packet);
        match packet {
            VLPUplinkPacket::GroundTestDeployPacket(GroundTestDeployPacket {
                pyro: pyro_selection,
                ..
            }) => {
                let finished = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

                let log_baro_fut = async {
                    let mut baro_ticker = Ticker::every(services.clock(), services.delay(), 5.0);
                    while !finished.lock(|s| *s.borrow()) {
                        baro_ticker.next().await;
                        if let Ok(reading) = barometer.read().await {
                            try_or_warn!(buffered_baro_logger.log(reading).await);
                        }
                    }

                    try_or_warn!(buffered_baro_logger.flush().await);
                };

                let buzzer_queue = &services.buzzer_queue;
                let fire_fut = async {
                    log_info!("3");

                    buzzer_queue.publish(3000, 50, 200);
                    buzzer_queue.publish(3000, 50, 200);
                    buzzer_queue.publish(3000, 50, 200);
                    services.delay.delay_ms(1000.0).await;

                    log_info!("2");
                    buzzer_queue.publish(3000, 50, 200);
                    buzzer_queue.publish(3000, 50, 200);
                    services.delay.delay_ms(1000.0).await;

                    log_info!("1");
                    buzzer_queue.publish(3000, 50, 200);
                    services.delay.delay_ms(1000.0).await;

                    log_info!("fire");
                    let fire_time = services.clock.now_ms();
                    pyro!(
                        device_manager,
                        pyro_selection,
                        pyro_ctrl.set_enable(true).await.unwrap()
                    );
                    services.delay.delay_ms(3000.0).await;
                    pyro!(
                        device_manager,
                        pyro_selection,
                        pyro_ctrl.set_enable(false).await.unwrap()
                    );
                    logger
                        .write(
                            &mut log_file_writer,
                            &GroundTestLog::FireEvent(FireEvent {
                                timestamp: fire_time,
                            }),
                        )
                        .await
                        .unwrap();
                    services.delay.delay_ms(10000.0).await;
                    finished.lock(|s| *s.borrow_mut() = true);
                    try_or_warn!(log_file_writer.flush().await);
                };

                join!(log_baro_fut, fire_fut);
            }
            _ => {
                // noop
            }
        }
    };
    let vlp_fut = async {
        if let Some(lora) = lora.as_mut() {
            vlp.run(
                services.delay(),
                lora,
                &config.lora,
                services.unix_clock(),
                &config.lora_key,
            )
            .await;
        }
    };

    let pyro_main_cont_fut = async {
        let mut cont = pyro!(
            device_manager,
            main_pyro,
            pyro_cont.read_continuity().await.unwrap()
        );

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_main_continuity = cont;
            });
            cont = pyro!(
                device_manager,
                main_pyro,
                pyro_cont.wait_continuity_change().await.unwrap()
            );
        }
    };

    let pyro_drogue_cont_fut = async {
        let mut cont = pyro!(
            device_manager,
            drogue_pyro,
            pyro_cont.read_continuity().await.unwrap()
        );

        loop {
            telemetry_packet_builder.update(|b| {
                b.pyro_drogue_continuity = cont;
            });
            cont = pyro!(
                device_manager,
                drogue_pyro,
                pyro_cont.wait_continuity_change().await.unwrap()
            );
        }
    };

    let arming_switch_fut = async {
        let mut armed = arming_switch.read_arming().await.unwrap();
        loop {
            arming_state.set_hardware_armed(armed);
            telemetry_packet_builder.update(|b| {
                b.hardware_armed = armed;
                b.software_armed = true;
            });
            armed = arming_switch.wait_arming_change().await.unwrap();
        }
    };

    let hardware_arming_beep_fut = async {
        let mut sub = arming_state.subscriber();
        let mut hardware_armed_debounced = arming_state.is_armed();
        if hardware_armed_debounced {
            services.buzzer_queue.publish(2000, 700, 300);
            services.buzzer_queue.publish(3000, 700, 300);
        }

        loop {
            let new_hardware_armed = sub.next_message_pure().await.hardware_armed;
            if !hardware_armed_debounced && new_hardware_armed {
                services.buzzer_queue.publish(2000, 700, 300);
                services.buzzer_queue.publish(3000, 700, 300);
            } else if hardware_armed_debounced && !new_hardware_armed {
                services.buzzer_queue.publish(3000, 700, 300);
                services.buzzer_queue.publish(2000, 700, 300);
            }
            hardware_armed_debounced = new_hardware_armed;
        }
    };

    #[allow(unused_must_use)]
    {
        join!(
            indicator_fut,
            vlp_tx_fut,
            vlp_rx_fut,
            vlp_fut,
            pyro_main_cont_fut,
            pyro_drogue_cont_fut,
            arming_switch_fut,
            buffered_baro_logger_runner.run(),
            arming_state_debounce_fut,
            hardware_arming_beep_fut,
            can_tx_avionics_status_fut,
            can_tx_unix_time_fut
        );
    }
    log_unreachable!()
}
