use core::cell::RefCell;

use crate::common::buzzer_queue::BuzzerQueue;
use crate::common::buzzer_queue::BuzzerTone;
use crate::common::unix_clock::UnixClock;
use crate::driver::barometer::ArchivedBaroReading;
use crate::driver::barometer::BaroReading;
use crate::{
    claim_devices,
    common::{device_manager::prelude::*, files::GROUND_TEST_LOG_FILE_TYPE, ticker::Ticker},
    create_serialized_logger, device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timestamp::UnixTimestamp},
};
use core::fmt::Write;
use defmt::{info, unwrap};
use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, blocking_mutex::Mutex as BlockingMutex};
use embedded_hal_async::delay::DelayNs;
use futures::join;
use heapless::String;
use lora_phy::mod_params::Bandwidth;
use lora_phy::mod_params::CodingRate;
use lora_phy::mod_params::SpreadingFactor;
use lora_phy::RxMode;
use rkyv::{Archive, Deserialize, Serialize};
use vlfs::VLFS;

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
struct FireEvent {
    pub timestamp: f64, // ms
}

type BaroReadingUnix = BaroReading<UnixTimestamp>;
type ArchivedBaroReadingUnix = ArchivedBaroReading<UnixTimestamp>;

create_serialized_logger!(
    GroundTestLogger, // this is the name of the struct
    GroundTestLoggerReader,
    GroundTestLog,
    16,
    (0, BaroReadingUnix),
    (1, FireEvent)
);

async fn fire_pyro(
    fs: &VLFS<impl Flash, impl Crc>,
    unix_clock: UnixClock<'_, impl Clock>,
    mut delay: impl DelayNs + Copy,
    ctrl: &mut impl PyroCtrl,
    baro: &mut impl Barometer,
    buzzer_queue: &BuzzerQueue<'_>,
) {
    let file = fs.create_file(GROUND_TEST_LOG_FILE_TYPE).await.unwrap();
    let writer = fs.open_file_for_write(file.id).await.unwrap();
    let mut logger = GroundTestLogger::new(writer);

    let finished = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    let logs_channel = Channel::<NoopRawMutex, GroundTestLog, 500>::new();
    let logger_fut = async {
        while !finished.lock(|s| *s.borrow()) {
            let log = logs_channel.receive().await;
            logger.log(log).await.unwrap();
        }
    };

    let mut baro_ticker = Ticker::every(unix_clock, delay.clone(), 5.0);
    let log_baro_fut = async {
        while !finished.lock(|s| *s.borrow()) {
            baro_ticker.next().await;
            if let Ok(reading) = baro.read().await {
                let reading = reading.to_unix_timestamp(unix_clock);
                logs_channel
                    .try_send(GroundTestLog::BaroReadingUnix(reading))
                    .unwrap();
            }
        }
    };

    let fire_fut = async {
        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        buzzer_queue.publish(BuzzerTone(None, 500));
        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        buzzer_queue.publish(BuzzerTone(None, 500));
        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;

        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        buzzer_queue.publish(BuzzerTone(None, 500));
        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;

        buzzer_queue.publish(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;

        logs_channel
            .try_send(GroundTestLog::FireEvent(FireEvent { timestamp: 0.0 }))
            .unwrap();
        unwrap!(ctrl.set_enable(true).await);
        delay.delay_ms(1000).await;
        unwrap!(ctrl.set_enable(false).await);
        delay.delay_ms(10000).await;
        finished.lock(|s| *s.borrow_mut() = true);
    };

    join!(logger_fut, log_baro_fut, fire_fut);

    let writer = logger.into_writer();
    writer.close().await.unwrap();
}

#[inline(never)]
pub async fn ground_test_avionics(
    fs: &VLFS<impl Flash, impl Crc>,
    unix_clock: UnixClock<'_, impl Clock>,
    buzzer_queue: &BuzzerQueue<'_>,
    device_manager: device_manager_type!(),
) -> ! {
    claim_devices!(
        device_manager,
        lora,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        status_indicator,
        barometer,
        arming_switch
    );

    let mut delay = device_manager.delay;
    let indicator_fut = async {
        loop {
            status_indicator.set_enable(true).await;
            delay.delay_ms(50).await;
            status_indicator.set_enable(false).await;
            delay.delay_ms(2000).await;
        }
    };

    let delay = device_manager.delay;
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
            if unwrap!(arming_switch.read_arming().await) {
                lora_message.push_str("Armed | ").unwrap();
            } else {
                lora_message.push_str("Disarmed | ").unwrap();
            }
            if unix_clock.ready() {
                lora_message.push_str("Clock Ready ").unwrap();
                let mut timestamp_str = String::<32>::new();
                core::write!(&mut timestamp_str, "{}", unix_clock.now_ms()).unwrap();
                lora_message.push_str(timestamp_str.as_str()).unwrap();
            } else {
                lora_message.push_str("Clock Not Ready").unwrap();
            }

            info!("{}", lora_message.as_str());

            unwrap!(lora.prepare_for_tx(
                &modulation_params,
                &mut tx_params,
                22,
                lora_message.as_bytes(),
            )
            .await);
            unwrap!(lora.tx().await);

            lora.prepare_for_rx(RxMode::Single(1000), &modulation_params, &rx_pkt_params)
                .await
                .unwrap();
            match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
                Ok((length,_)) => {
                    let data = &receiving_buffer[0..(length as usize)];
                    info!("Received {} bytes", length);
                    if data == b"VLF4 fire 1" {
                        info!("Firing pyro 1");
                        fire_pyro(
                            fs,
                            unix_clock,
                            delay,
                            &mut pyro1_ctrl,
                            &mut barometer,
                            buzzer_queue,
                        )
                        .await;
                    } else if data == b"VLF4 fire 2" {
                        info!("Firing pyro 2");
                        fire_pyro(
                            fs,
                            unix_clock,
                            delay,
                            &mut pyro2_ctrl,
                            &mut barometer,
                            buzzer_queue,
                        )
                        .await;
                    }
                }
                Err(lora_error) => {
                    info!("Radio Error: {:?}", lora_error);
                }
            }
        }
    };

    join!(indicator_fut, avionics_fut);
    defmt::unreachable!()
}
