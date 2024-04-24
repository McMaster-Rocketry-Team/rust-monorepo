use defmt::{info, warn};
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

    claim_devices!(device_manager, radio_phy);

    loop {
        
        match radio_phy.rx().await {
            Ok(data) => {
                info!(
                    "Received {} bytes",
                    data.0.len
                );
                
                if let Ok(archived) = check_archived_root::<BeaconData>(&data.1) {
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
