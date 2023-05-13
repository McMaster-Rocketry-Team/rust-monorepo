use defmt::{info, warn};
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    mod_traits::RadioKind,
    LoRa,
};
use vlfs::{Crc, Flash, VLFS};

use crate::{
    allocator::HEAP,
    beacon::beacon_data::BeaconData,
    driver::timer::{DelayUsWrapper, Timer},
};
use rkyv::{check_archived_root, Deserialize};

#[inline(never)]
pub async fn beacon_receiver(
    timer: impl Timer,
    _fs: &VLFS<impl Flash, impl Crc>,
    radio_kind: impl RadioKind + 'static,
) -> ! {
    // Init 1KiB heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut lora = LoRa::new(radio_kind, false, &mut DelayUsWrapper(timer))
        .await
        .unwrap();

    let modulation_params = lora
        .create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            915_000_000,
        )
        .unwrap();
    let rx_params = lora
        .create_rx_packet_params(8, false, 255, true, false, &modulation_params)
        .unwrap();

    let mut buffer = [0u8; 256];
    loop {
        lora.prepare_for_rx(
            &modulation_params,
            &rx_params,
            None,
            true,
            true,
            4,
            0x00FFFFFFu32,
        )
        .await
        .unwrap();

        match lora.rx(&rx_params, &mut buffer).await {
            Ok((received_len, status)) => {
                info!(
                    "Received {} bytes, snr: {}, rssi: {}",
                    received_len, status.snr, status.rssi
                );
                let buffer = &buffer[..(received_len as usize)];
                if let Ok(archived) = check_archived_root::<BeaconData>(buffer) {
                    let d: BeaconData = archived.deserialize(&mut rkyv::Infallible).unwrap();
                    info!("BeaconData: {}", d);
                } else {
                    warn!("Invalid BeaconData");
                }
            }
            Err(err) => {
                info!("Error: {:?}", err);
            }
        }
    }
}
