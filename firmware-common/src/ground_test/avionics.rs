use core::cell::RefCell;

use crate::driver::barometer::ArchivedBaroReading;
use crate::driver::barometer::BaroReading;
use crate::{
    claim_devices,
    common::{device_manager::prelude::*, file_types::GROUND_TEST_LOG_FILE_TYPE, ticker::Ticker},
    create_serialized_enum, device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timestamp::UnixTimestamp},
};
use core::fmt::Write;
use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, blocking_mutex::Mutex as BlockingMutex};
use futures::join;
use heapless::String;
use lora_phy::mod_params::Bandwidth;
use lora_phy::mod_params::CodingRate;
use lora_phy::mod_params::SpreadingFactor;
use lora_phy::RxMode;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
struct FireEvent {
    pub timestamp: f64, // ms
}

type BaroReadingUnix = BaroReading<UnixTimestamp>;
type ArchivedBaroReadingUnix = ArchivedBaroReading<UnixTimestamp>;

create_serialized_enum!(
    GroundTestLogger, // this is the name of the struct
    GroundTestLoggerReader,
    GroundTestLog,
    (0, BaroReadingUnix),
    (1, FireEvent)
);

async fn fire_pyro(
    services: system_services_type!(),
    ctrl: &mut impl PyroCtrl,
    baro: &mut impl Barometer,
) {
    let file = services
        .fs
        .create_file(GROUND_TEST_LOG_FILE_TYPE)
        .await
        .unwrap();
    let writer = services.fs.open_file_for_write(file.id).await.unwrap();
    let mut logger = GroundTestLogger::new(writer);

    let finished = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    let logs_channel = Channel::<NoopRawMutex, GroundTestLog, 500>::new();
    let logger_fut = async {
        while !finished.lock(|s| *s.borrow()) {
            let log = logs_channel.receive().await;
            logger.write(&log).await.unwrap();
        }
    };

    let mut baro_ticker = Ticker::every(services.unix_clock(), services.delay(), 5.0);
    let log_baro_fut = async {
        while !finished.lock(|s| *s.borrow()) {
            baro_ticker.next().await;
            if let Ok(reading) = baro.read().await {
                let reading = reading.to_unix_timestamp(services.unix_clock());
                logs_channel
                    .try_send(GroundTestLog::BaroReadingUnix(reading))
                    .unwrap();
            }
        }
    };

    let buzzer_queue = &services.buzzer_queue;
    let fire_fut = async {
        log_info!("3");

        buzzer_queue.publish(3000, 50, 100);
        buzzer_queue.publish(3000, 50, 100);
        buzzer_queue.publish(3000, 50, 100);
        services.delay.delay_ms(1000).await;

        log_info!("2");
        buzzer_queue.publish(3000, 50, 100);
        buzzer_queue.publish(3000, 50, 100);
        services.delay.delay_ms(1000).await;

        log_info!("1");
        buzzer_queue.publish(3000, 50, 100);
        services.delay.delay_ms(1000).await;

        log_info!("fire");
        logs_channel
            .try_send(GroundTestLog::FireEvent(FireEvent {
                timestamp: services.unix_clock.now_ms(),
            }))
            .unwrap();
        ctrl.set_enable(true).await.unwrap();
        services.delay.delay_ms(2000).await;
        ctrl.set_enable(false).await.unwrap();
        services.delay.delay_ms(10000).await;
        finished.lock(|s| *s.borrow_mut() = true);
    };

    join!(logger_fut, log_baro_fut, fire_fut);

    let writer = logger.into_writer();
    writer.close().await.unwrap();
}

#[inline(never)]
pub async fn ground_test_avionics(
    device_manager: device_manager_type!(),
    services: system_services_type!(),
) -> ! {
    claim_devices!(
        device_manager,
        lora,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        barometer,
        arming_switch,
        indicators
    );

    log_info!("resetting barometer");
    barometer.reset().await.unwrap();

    let indicator_fut = indicators.run([], [50, 2000], []);

    let avionics_fut = async {
        let modulation_params = lora
            .create_modulation_params(
                SpreadingFactor::_12,
                Bandwidth::_250KHz,
                CodingRate::_4_8,
                903_900_000,
            )
            .unwrap();
        let mut tx_params = lora
            .create_tx_packet_params(4, false, false, false, &modulation_params)
            .unwrap();
        let rx_pkt_params = lora
            .create_rx_packet_params(4, false, 50, false, false, &modulation_params)
            .unwrap();
        let mut receiving_buffer = [0u8; 50];
        loop {
            let mut lora_message = String::<100>::new();
            match pyro1_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 1: Cont | ").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 1: No Cont | ").unwrap(),
                Err(_) => lora_message.push_str("Pyro 1: Error | ").unwrap(),
            };
            match pyro2_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 2: Cont | ").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 2: No Cont | ").unwrap(),
                Err(_) => lora_message.push_str("Pyro 2: Error | ").unwrap(),
            };
            if arming_switch.read_arming().await.unwrap() {
                lora_message.push_str("Armed | ").unwrap();
            } else {
                lora_message.push_str("Disarmed | ").unwrap();
            }
            if services.unix_clock.ready() {
                lora_message.push_str("Clock Ready ").unwrap();
                let mut timestamp_str = String::<32>::new();
                core::write!(&mut timestamp_str, "{}", services.unix_clock.now_ms()).unwrap();
                lora_message.push_str(timestamp_str.as_str()).unwrap();
            } else {
                lora_message.push_str("Clock Not Ready").unwrap();
            }

            log_info!("{}", lora_message.as_str());

            lora.prepare_for_tx(
                &modulation_params,
                &mut tx_params,
                9,
                lora_message.as_bytes(),
            )
            .await
            .unwrap();
            lora.tx().await.unwrap();

            lora.prepare_for_rx(RxMode::Single(1000), &modulation_params, &rx_pkt_params)
                .await
                .unwrap();

            match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
                Ok((length, _)) => {
                    let data = &receiving_buffer[0..(length as usize)];
                    log_info!("Received {} bytes", length);
                    if data == b"VLF4 fire 1" {
                        log_info!("Firing pyro 1");
                        fire_pyro(services, &mut pyro1_ctrl, &mut barometer).await;
                    } else if data == b"VLF4 fire 2" {
                        log_info!("Firing pyro 2");
                        fire_pyro(services, &mut pyro2_ctrl, &mut barometer).await;
                    }
                }
                Err(lora_error) => {
                    log_info!("Radio Error: {:?}", lora_error);
                }
            }
        }
    };

    join!(indicator_fut, avionics_fut);
    log_unreachable!()
}
