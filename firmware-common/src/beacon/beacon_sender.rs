use heapless::Vec;
use lora_phy::mod_params::{Bandwidth, CodingRate, SpreadingFactor};
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};
use vlfs::{AsyncWriter, Crc, Flash, VLFS};

use crate::{
    beacon::beacon_data::BeaconData,
    claim_devices,
    common::{
        device_manager::prelude::*, files::BEACON_SENDER_LOG_FILE_TYPE, gps_parser::GPSParser,
    },
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator},
};
use core::{cell::RefCell, fmt::Write};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex as BlockingMutex};
use futures::future::join3;
use heapless::String;

#[inline(never)]
pub async fn beacon_sender(
    fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
    use_lora: bool,
) -> ! {
    claim_devices!(device_manager, gps, lora, red_indicator, green_indicator);
    let clock = device_manager.clock;
    let mut lora = if use_lora { Some(lora) } else { None };

    let file = fs.create_file(BEACON_SENDER_LOG_FILE_TYPE).await.unwrap();
    let mut log_file = fs.open_file_for_write(file.id).await.unwrap();
    log_file
        .extend_from_slice(b"\n\nBeacon Started =================\n")
        .await
        .ok();

    let satellites_count = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(0u32));
    let locked = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    let gps_parser = GPSParser::new();
    // todo log gps location to file

    let gps_fut = gps_parser.run(&mut gps);

    let mut delay = device_manager.delay;
    let beacon_fut = async {
        loop {
            let nmea = gps_parser.get_nmea();

            satellites_count.lock(|v| v.replace(nmea.num_of_fix_satellites as u32));
            locked.lock(|v| v.replace(nmea.lat_lon.is_some()));

            if let Some(lora) = &mut lora {
                let mut buffer = Vec::<u8, 222>::new();
                unsafe {
                    for _ in 0..core::mem::size_of::<<BeaconData as Archive>::Archived>() {
                        buffer.push_unchecked(0);
                    }
                }
                let mut serializer = BufferSerializer::new(buffer);
                let beacon_data = BeaconData {
                    satellite_count: Some(nmea.num_of_fix_satellites as u8),
                    longitude: nmea.lat_lon.map(|v| v.1 as f32),
                    latitude: nmea.lat_lon.map(|v| v.0 as f32),
                };
                serializer.serialize_value(&beacon_data).unwrap();
                let buffer = serializer.into_inner();

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
                lora.prepare_for_tx(&modulation_params, &mut tx_params, 22, &buffer)
                    .await;
                lora.tx().await;
            }

            let mut log_str = String::<32>::new();
            core::write!(&mut log_str, "{} | Beacon send!\n", clock.now_ms()).unwrap();
            log_file.extend_from_slice(log_str.as_bytes()).await.ok();
            log_info!(
                "{}",
                log_str
                    .as_str()
                    .trim_end_matches(|c| c == '\r' || c == '\n')
            );

            delay.delay_ms(1000).await;
        }
    };

    let mut delay = device_manager.delay;
    let indicator_fut = async {
        loop {
            let satellites_count: u32 = satellites_count.lock(|v| *v.borrow());
            let locked: bool = locked.lock(|v| *v.borrow());

            red_indicator.set_enable(locked).await;

            for _ in 0..(satellites_count + 1) {
                green_indicator.set_enable(true).await;
                delay.delay_ms(20).await;
                green_indicator.set_enable(false).await;
                delay.delay_ms(50).await;
            }

            delay.delay_ms(1000).await;
        }
    };

    join3(beacon_fut, indicator_fut, gps_fut).await;
    log_unreachable!()
}
