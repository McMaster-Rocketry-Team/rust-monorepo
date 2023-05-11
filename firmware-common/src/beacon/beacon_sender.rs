use defmt::{info, unwrap};
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    mod_traits::RadioKind,
    LoRa,
};
use vlfs::{io_traits::AsyncWriter, Crc, Flash, VLFS};

use crate::{
    beacon::beacon_data::BeaconData,
    common::gps_parser::GPSParser,
    driver::{
        gps::GPS,
        timer::{DelayUsWrapper, Timer},
    },
};
use core::fmt::Write;
use heapless::String;
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};

static BEACON_SENDER_LOG_FILE_ID: u64 = 1;
static BEACON_SENDER_LOG_FILE_TYPE: u16 = 1;

#[inline(never)]
pub async fn beacon_sender(
    timer: impl Timer,
    fs: &VLFS<impl Flash + defmt::Format, impl Crc>,
    mut gps_parser: GPSParser<impl GPS>,
    radio_kind: impl RadioKind + 'static,
) -> ! {
    let mut lora = LoRa::new(radio_kind, false, &mut DelayUsWrapper(timer))
        .await
        .unwrap();

    if !fs.exists(BEACON_SENDER_LOG_FILE_ID).await {
        unwrap!(
            fs.create_file(BEACON_SENDER_LOG_FILE_ID, BEACON_SENDER_LOG_FILE_TYPE)
                .await
        );
    }
    let mut log_file = unwrap!(fs.open_file_for_write(BEACON_SENDER_LOG_FILE_ID).await);
    log_file
        .extend_from_slice(b"\n\nBeacon Started =================\n")
        .await
        .ok();

    loop {
        while let Some(sentence) = gps_parser.update_one() {
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
            info!("{}", log_str.as_str());
        }
        let beacon_data = BeaconData {
            satellite_count: Some(gps_parser.nmea.satellites().len() as u8),
            longitude: gps_parser.nmea.longitude.map(|v| v as f32),
            latitude: gps_parser.nmea.latitude.map(|v| v as f32),
        };
        let mut serializer = BufferSerializer::new([0u8; 32]);
        serializer.serialize_value(&beacon_data).unwrap();
        let buffer = serializer.into_inner();
        let buffer = &buffer[..core::mem::size_of::<<BeaconData as Archive>::Archived>()];

        let modulation_params = lora
            .create_modulation_params(
                SpreadingFactor::_12,
                Bandwidth::_250KHz,
                CodingRate::_4_5,
                915_000_000,
            )
            .unwrap();
        let mut tx_params = lora
            .create_tx_packet_params(8, false, true, false, &modulation_params)
            .unwrap();
        lora.prepare_for_tx(&modulation_params, 0, true)
            .await
            .unwrap();
        lora.tx(&modulation_params, &mut tx_params, buffer, 0xFFFFFF)
            .await
            .unwrap();

        let mut log_str = String::<32>::new();
        core::write!(&mut log_str, "{} | Beacon send!\n", timer.now_mills()).unwrap();
        log_file.extend_from_slice(log_str.as_bytes()).await.ok();
        info!("{}", log_str.as_str());

        lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
        timer.sleep(2000).await;
    }
}
