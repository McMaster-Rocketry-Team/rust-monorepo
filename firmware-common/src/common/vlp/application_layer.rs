use core::{mem::size_of, ops::Deref};

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};
use embedded_hal_async::delay::DelayNs;
use rkyv::{
    check_archived_root,
    ser::{serializers::BufferSerializer, Serializer},
    AlignedBytes, Archive, Deserialize, Serialize,
};

use super::{Priority, VLPError, VLPSocket};
use crate::{common::telemetry::telemetry_data::TelemetryData, driver::{radio::RadioPhy}};
use core::fmt::Write;
use heapless::{String, Vec};

pub trait RadioApplicationPackage: Sized {
    fn encode(self) -> Vec<u8, 222>;
    fn decode(package: Vec<u8, 222>) -> Option<Self>;
}

pub trait RadioApplicationClient {
    type Error: defmt::Format;

    async fn run<
        R: RawMutex,
        TXP: RadioApplicationPackage,
        RXP: RadioApplicationPackage,
        D: DelayNs,
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        delay: D,
        radio_tx: Receiver<R, TXP, TXN>,
        radio_rx: Sender<R, RXP, RXN>,
    ) -> !;
}

impl<P: RadioPhy> RadioApplicationClient for VLPSocket<P> {
    type Error = P::Error;

    async fn run<
        'a,
        'b,
        R: RawMutex,
        TXP: RadioApplicationPackage,
        RXP: RadioApplicationPackage,
        D: DelayNs,
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        mut delay: D,
        radio_tx: Receiver<'a, R, TXP, TXN>,
        radio_rx: Sender<'b, R, RXP, RXN>,
    ) -> ! {
        loop {
            delay.delay_ms(100).await;
            match self.prio {
                Priority::Driver => {
                    if let Ok(tx_package) = radio_tx.try_receive() {
                        self.transmit(tx_package.encode()).await;
                    } else {
                        self.handoff().await;
                    }
                }
                Priority::Listener => {
                    if let Ok(Some(data)) = self.receive().await {
                        if let Some(decoded) = RXP::decode(data) {
                            radio_rx.send(decoded).await;
                        } else {
                            log_warn!("VLP: Received invalid payload");
                        }
                    }
                }
            }
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, defmt::Format)]
#[archive(check_bytes)]
pub enum ApplicationLayerRxPackage {
    // stand the rocket vertically so VLF can know which angle it is mounted at
    VerticalCalibration,
    ClearStorage,
    SoftArming(bool),
}

// FIXME use rkyv
impl RadioApplicationPackage for ApplicationLayerRxPackage {
    fn encode(self) -> Vec<u8, 222> {
        let mut buffer = Vec::<u8, 222>::new();
        unsafe {
            buffer.push_unchecked(0x69);
            match self {
                ApplicationLayerRxPackage::VerticalCalibration => {
                    buffer.push_unchecked(0x00);
                },
                ApplicationLayerRxPackage::ClearStorage => {
                    buffer.push_unchecked(0x01);
                },
                ApplicationLayerRxPackage::SoftArming(true) => {
                    buffer.push_unchecked(0x02);
                },
                ApplicationLayerRxPackage::SoftArming(false) => {
                    buffer.push_unchecked(0x03);
                },
            }
        }
        buffer
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerRxPackage> {
        log_info!("decoding package: {:?}", package.as_slice());
        if package.len() != 2 || package[0] != 0x69 {
            log_warn!("VLP: decode failed: not start with 0x69");
            return None;
        }
        match package[1] {
            0x00 => Some(ApplicationLayerRxPackage::VerticalCalibration),
            0x01 => Some(ApplicationLayerRxPackage::ClearStorage),
            0x02 => Some(ApplicationLayerRxPackage::SoftArming(true)),
            0x03 => Some(ApplicationLayerRxPackage::SoftArming(false)),
            _ => {
                log_warn!("VLP: decode failed: unknown command");
                None
            }
        }
    }
}

#[derive(Debug, Clone, defmt::Format)]

pub enum ApplicationLayerTxPackage {
    Telemetry(TelemetryData),
}

// FIXME use rkyv
impl RadioApplicationPackage for ApplicationLayerTxPackage {
    fn encode(self) -> Vec<u8, 222> {
        match self {
            ApplicationLayerTxPackage::Telemetry(telemetry) => {
                let mut buffer = Vec::<u8, 222>::new();
                unsafe {
                    buffer.push_unchecked(0x69);
                    telemetry.max_altitude.to_be_bytes().iter().for_each(|b| {
                        buffer.push_unchecked(*b);
                    });
                    if let Some((lat, lon)) = telemetry.lat_lon {
                        lat.to_be_bytes().iter().for_each(|b| {
                            buffer.push_unchecked(*b);
                        });
                        lon.to_be_bytes().iter().for_each(|b| {
                            buffer.push_unchecked(*b);
                        });
                    } else {
                        for _ in 0..16 {
                            buffer.push_unchecked(0);
                        }
                    }
                    buffer.push_unchecked(if telemetry.software_armed {1}else {0});
                    buffer.push_unchecked(if telemetry.hardware_armed {1}else {0});
                }

                buffer
            }
        }
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerTxPackage> {
        log_info!("decoding package: {:?}", package.as_slice());
        if package.len() != 4 + 8 + 8+2 + 1 || package[0] != 0x69 {
            log_warn!("VLP: decode failed: not start with 0x69");
            return None;
        }
        let bytes = &package.as_slice()[1..];
        let max_altitude = f32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let lat = f64::from_be_bytes(bytes[4..12].try_into().unwrap());
        let lon = f64::from_be_bytes(bytes[12..20].try_into().unwrap());
        let software_armed = bytes[20] == 1;
        let hardware_armed = bytes[21] == 1;

        let mut telemetry = TelemetryData::default();
        telemetry.max_altitude = max_altitude;
        telemetry.lat_lon = Some((lat, lon));
        telemetry.software_armed = software_armed;
        telemetry.hardware_armed = hardware_armed;

        Some(ApplicationLayerTxPackage::Telemetry(telemetry))
    }
}
