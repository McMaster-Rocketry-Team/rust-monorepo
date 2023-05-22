use defmt::{info, unwrap};
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    mod_traits::RadioKind,
};
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};
use vlfs::{io_traits::AsyncWriter, Crc, Flash, VLFS};

use crate::{
    beacon::beacon_data::BeaconData,
    claim_devices,
    common::{
        device_manager::prelude::*, files::BEACON_SENDER_LOG_FILE_TYPE, gps_parser::GPSParser,
    },
    device_manager_type,
    driver::{
        gps::GPS,
        indicator::Indicator,
        timer::{DelayUsWrapper, Timer},
    },
};
use core::{cell::RefCell, fmt::Write};
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex as BlockingMutex};
use futures::future::join;
use heapless::String;

#[inline(never)]
pub async fn beacon_sender(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
    use_lora: bool,
) -> ! {
    claim_devices!(device_manager, gps, lora, error_indicator, status_indicator);
    let timer = device_manager.timer;
    let mut lora = if use_lora { lora.as_mut() } else { None };
    if let Some(lora) = &mut lora {
        lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
    }

    let file_id = unwrap!(fs.create_file(BEACON_SENDER_LOG_FILE_TYPE).await);
    let mut log_file = unwrap!(fs.open_file_for_write(file_id).await);
    log_file
        .extend_from_slice(b"\n\nBeacon Started =================\n")
        .await
        .ok();

    let satellites_count = BlockingMutex::<CriticalSectionRawMutex, _>::new(RefCell::new(0u32));
    let locked = BlockingMutex::<CriticalSectionRawMutex, _>::new(RefCell::new(false));

    let beacon_fut = async {
        let mut gps_parser = GPSParser::new(gps);
        loop {
            while let Some((sentence, _)) = gps_parser.update_one() {
                let mut log_str = String::<128>::new();
                core::write!(
                    &mut log_str,
                    "{} | NMEA received at {}: {}\n",
                    timer.now_mills(),
                    sentence.timestamp,
                    sentence
                        .sentence
                        .as_str()
                        .trim_end_matches(|c| c == '\r' || c == '\n')
                )
                .unwrap();
                log_file.extend_from_slice(log_str.as_bytes()).await.ok();
                info!(
                    "{}",
                    log_str
                        .as_str()
                        .trim_end_matches(|c| c == '\r' || c == '\n')
                );
            }

            satellites_count.lock(|v| v.replace(gps_parser.nmea.satellites().len() as u32));
            locked.lock(|v| v.replace(gps_parser.nmea.longitude.is_some()));

            if let Some(lora) = &mut lora {
                let mut serializer = BufferSerializer::new([0u8; 32]);
                let beacon_data = BeaconData {
                    satellite_count: Some(gps_parser.nmea.satellites().len() as u8),
                    longitude: gps_parser.nmea.longitude.map(|v| v as f32),
                    latitude: gps_parser.nmea.latitude.map(|v| v as f32),
                };
                serializer.serialize_value(&beacon_data).unwrap();
                let buffer = serializer.into_inner();
                let buffer = &buffer[..core::mem::size_of::<<BeaconData as Archive>::Archived>()];

                let modulation_params = lora
                    .create_modulation_params(
                        SpreadingFactor::_12,
                        Bandwidth::_250KHz,
                        CodingRate::_4_8,
                        915_000_000,
                    )
                    .unwrap();
                let mut tx_params = lora
                    .create_tx_packet_params(8, false, true, false, &modulation_params)
                    .unwrap();
                lora.prepare_for_tx(&modulation_params, 12, true)
                    .await
                    .unwrap();
                lora.tx(&modulation_params, &mut tx_params, buffer, 0xFFFFFF)
                    .await
                    .unwrap();
            }

            let mut log_str = String::<32>::new();
            core::write!(&mut log_str, "{} | Beacon send!\n", timer.now_mills()).unwrap();
            log_file.extend_from_slice(log_str.as_bytes()).await.ok();
            info!(
                "{}",
                log_str
                    .as_str()
                    .trim_end_matches(|c| c == '\r' || c == '\n')
            );

            if let Some(lora) = &mut lora {
                lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
            }
            timer.sleep(1000.0).await;
        }
    };

    let indicator_fut = async {
        loop {
            let satellites_count: u32 = satellites_count.lock(|v| *v.borrow());
            let locked: bool = locked.lock(|v| *v.borrow());

            error_indicator.set_enable(locked).await;

            for _ in 0..(satellites_count + 1) {
                status_indicator.set_enable(true).await;
                timer.sleep(20.0).await;
                status_indicator.set_enable(false).await;
                timer.sleep(50.0).await;
            }

            timer.sleep(1000.0).await;
        }
    };

    join(beacon_fut, indicator_fut).await;
    defmt::unreachable!()
}
