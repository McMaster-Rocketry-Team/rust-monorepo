use lora_phy::{
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    RxMode,
};
use vlfs::{Crc, Flash, VLFS};

use crate::{
    allocator::HEAP, beacon::beacon_data::BeaconData, claim_devices,
    common::device_manager::prelude::*, device_manager_type,
};
use rkyv::{check_archived_root, Deserialize};

#[inline(never)]
pub async fn beacon_receiver(
    _fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
) -> ! {
    // Init 1KiB heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    claim_devices!(device_manager, lora);

    let modulation_params = lora
        .create_modulation_params(
            SpreadingFactor::_12,
            Bandwidth::_250KHz,
            CodingRate::_4_8,
            903_900_000,
        )
        .unwrap();
    let rx_pkt_params = lora
        .create_rx_packet_params(4, false, 50, false, false, &modulation_params)
        .unwrap();
    let mut receiving_buffer = [0u8; 222];
    loop {
        lora.prepare_for_rx(RxMode::Single(1000), &modulation_params, &rx_pkt_params)
            .await
            .unwrap();
        match lora.rx(&rx_pkt_params, &mut receiving_buffer).await {
            Ok((length, _)) => {
                log_info!("Received {} bytes", length);
                let data = &receiving_buffer[0..(length as usize)];
                if let Ok(archived) = check_archived_root::<BeaconData>(data) {
                    let d: BeaconData = archived.deserialize(&mut rkyv::Infallible).unwrap();
                    log_info!("BeaconData: {:?}", d);
                } else {
                    log_warn!("Invalid BeaconData");
                }
            }
            Err(err) => {
                log_info!("Error: {:?}", err);
            }
        }
    }
}
