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
use futures::future::join3;
use heapless::String;

#[inline(never)]
pub async fn gcm_main(fs: &VLFS<impl Flash, impl Crc>, device_manager: device_manager_type!()) {}
