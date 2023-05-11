use defmt::info;
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    mod_traits::RadioKind,
    LoRa,
};
use vlfs::{Crc, Flash, VLFS};

use crate::{
    beacon::beacon_data::BeaconData,
    driver::{
        gps::GPS,
        timer::{DelayUsWrapper, Timer},
    },
};
use rkyv::{
    ser::{serializers::BufferSerializer, Serializer},
    Archive,
};

#[inline(never)]
pub async fn beacon_sender(
    timer: impl Timer,
    _fs: &VLFS<impl Flash, impl Crc>,
    mut gps: impl GPS,
    radio_kind: impl RadioKind + 'static,
) -> ! {
    // gps.reset().await;

    let mut lora = LoRa::new(radio_kind, false, &mut DelayUsWrapper(timer))
        .await
        .unwrap();

    loop {
        let nmea = gps.receive().await;
        let beacon_data = BeaconData {
            satellite_count: nmea.satellites_visible,
            longitude: nmea.longitude,
            latitude: nmea.latitude,
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
        info!("beacon send! {=[u8]:X}", buffer);

        lora.sleep(&mut DelayUsWrapper(timer)).await.unwrap();
        timer.sleep(1000).await;
    }
}
