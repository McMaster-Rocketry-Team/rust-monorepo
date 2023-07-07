use defmt::{info, unwrap};
use heapless::Vec;
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
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
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
    claim_devices!(
        device_manager,
        gps,
        vlp_phy,
        error_indicator,
        status_indicator
    );
    let timer = device_manager.timer;
    let mut vlp_phy = if use_lora { Some(vlp_phy) } else { None };

    let file_id = unwrap!(fs.create_file(BEACON_SENDER_LOG_FILE_TYPE).await);
    let mut log_file = unwrap!(fs.open_file_for_write(file_id).await);
    log_file
        .extend_from_slice(b"\n\nBeacon Started =================\n")
        .await
        .ok();

    let satellites_count = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(0u32));
    let locked = BlockingMutex::<NoopRawMutex, _>::new(RefCell::new(false));

    let gps_parser = GPSParser::new(timer);
    // todo log gps location to file

    let gps_fut = gps_parser.run(&mut gps);

    let beacon_fut = async {
        loop {
            let nmea = gps_parser.get_nmea();

            satellites_count.lock(|v| v.replace(nmea.num_of_fix_satellites));
            locked.lock(|v| v.replace(nmea.lat_lon.is_some()));

            if let Some(vlp_phy) = &mut vlp_phy {
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

                vlp_phy.tx(&buffer).await;
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

    join3(beacon_fut, indicator_fut, gps_fut).await;
    defmt::unreachable!()
}
