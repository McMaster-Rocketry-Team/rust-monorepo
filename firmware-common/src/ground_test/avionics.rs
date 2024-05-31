use core::cell::RefCell;

use crate::driver::barometer::ArchivedBaroReading;
use crate::driver::barometer::BaroReading;
use crate::{
    claim_devices,
    common::{device_manager::prelude::*, files::GROUND_TEST_LOG_FILE_TYPE, ticker::Ticker},
    create_serialized_logger, device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timestamp::UnixTimestamp},
};
use defmt::{info, unwrap};
use embassy_sync::pubsub::Publisher;
use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    blocking_mutex::Mutex as BlockingMutex,
    pubsub::PubSubChannel,
};
use embedded_hal_async::delay::DelayNs;
use futures::join;
use heapless::String;
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
    clock: impl Clock,
    mut delay: impl DelayNs + Copy,
    ctrl: &mut impl PyroCtrl,
    mut baro: &mut impl Barometer,
    buzzer_pub: &Publisher<'_, NoopRawMutex, BuzzerTone, 7, 1, 1>,
) {
    let file = fs.create_file(GROUND_TEST_LOG_FILE_TYPE).await.unwrap();
    let writer = fs.open_file_for_write(file.id).await.unwrap();
    let mut logger = GroundTestLogger::new(writer);

    let finished = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    let mut baro_ticker = Ticker::every(clock, delay.clone(), 5.0);
    let log_baro_fut = async {
        while !finished.lock(|s| *s.borrow()) {
            baro_ticker.next().await;
            if let Ok(reading) = baro.read().await {
                // logger
                //     .log(GroundTestLog::BaroReadingUnix(reading))
                //     .await
                //     .unwrap();
            }
        }
    };

    let fire_fut = async {
        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        buzzer_pub.publish_immediate(BuzzerTone(None, 500));
        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        buzzer_pub.publish_immediate(BuzzerTone(None, 500));
        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;

        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        buzzer_pub.publish_immediate(BuzzerTone(None, 500));
        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;

        buzzer_pub.publish_immediate(BuzzerTone(Some(3000), 500));
        delay.delay_ms(1000).await;
        logger
            .log(GroundTestLog::FireEvent(FireEvent { timestamp: 0.0 }))
            .await
            .unwrap();
        unwrap!(ctrl.set_enable(true).await);
        delay.delay_ms(1000).await;
        unwrap!(ctrl.set_enable(false).await);
        delay.delay_ms(10000).await;
        finished.lock(|s| *s.borrow_mut() = true);
    };

    join!(log_baro_fut, fire_fut);

    let writer = logger.into_writer();
    writer.close().await.unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BuzzerTone(Option<u32>, u32);

#[inline(never)]
pub async fn ground_test_avionics(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) -> ! {
    claim_devices!(
        device_manager,
        radio_phy,
        pyro1_cont,
        pyro1_ctrl,
        pyro2_cont,
        pyro2_ctrl,
        status_indicator,
        barometer,
        buzzer
    );

    let shared_buzzer_channel = PubSubChannel::<NoopRawMutex, BuzzerTone, 7, 1, 1>::new();

    let mut delay = device_manager.delay;
    let shared_buzzer_fut = async {
        let mut sub = shared_buzzer_channel.subscriber().unwrap();
        loop {
            let tone = sub.next_message_pure().await;
            if let Some(frequency) = tone.0 {
                buzzer.play(frequency, tone.1).await;
            } else {
                delay.delay_ms(tone.1).await;
            }
        }
    };
    let buzzer_pub: embassy_sync::pubsub::Publisher<NoopRawMutex, BuzzerTone, 7, 1, 1> =
        shared_buzzer_channel.publisher().unwrap();

    let mut delay = device_manager.delay;
    let indicator_fut = async {
        loop {
            status_indicator.set_enable(true).await;
            delay.delay_ms(50).await;
            status_indicator.set_enable(false).await;
            delay.delay_ms(2000).await;
        }
    };

    let mut delay = device_manager.delay;
    let avionics_fut = async {
        loop {
            let mut lora_message = String::<50>::new();
            match pyro1_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 1: Cont | ").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 1: No Cont | ").unwrap(),
                Err(_) => lora_message.push_str("Pyro 1: Error | ").unwrap(),
            };
            match pyro2_cont.read_continuity().await {
                Ok(true) => lora_message.push_str("Pyro 2: Cont").unwrap(),
                Ok(false) => lora_message.push_str("Pyro 2: No Cont").unwrap(),
                Err(_) => lora_message.push_str("Pyro 2: Error").unwrap(),
            };

            info!("{}", lora_message.as_str());

            radio_phy.tx(lora_message.as_bytes()).await;

            match radio_phy.rx_with_timeout(1000).await {
                Ok(Some(data)) => {
                    info!("Received {} bytes", data.0.len);
                    let rx_buffer = data.1.as_slice();
                    if rx_buffer == b"VLF3 fire 1" {
                        info!("Firing pyro 1");
                        unwrap!(pyro1_ctrl.set_enable(true).await);
                        delay.delay_ms(1000).await;
                        unwrap!(pyro1_ctrl.set_enable(false).await);
                    } else if rx_buffer == b"VLF3 fire 2" {
                        info!("Firing pyro 2");
                        unwrap!(pyro2_ctrl.set_enable(true).await);
                        delay.delay_ms(1000).await;
                        unwrap!(pyro2_ctrl.set_enable(false).await);
                    }
                }
                Ok(None) => {
                    info!("rx Timeout");
                }
                Err(lora_error) => {
                    info!("Radio Error: {:?}", lora_error);
                }
            }

            delay.delay_ms(2000).await;
        }
    };

    join!(indicator_fut, avionics_fut, shared_buzzer_fut);
    defmt::unreachable!()
}
