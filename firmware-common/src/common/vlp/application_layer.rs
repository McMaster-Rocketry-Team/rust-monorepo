use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    channel::{Receiver, Sender},
};
use rkyv::{
    check_archived_root,
    ser::{serializers::BufferSerializer, Serializer},
    Archive, Deserialize, Serialize,
};

use super::{phy::VLPPhy, Priority, VLPError, VLPSocket};
use crate::common::telemetry::telemetry_data::TelemetryData;
use heapless::Vec;

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
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        radio_tx: Receiver<R, TXP, TXN>,
        radio_rx: Sender<R, RXP, RXN>,
    ) -> !;
}

impl<P: VLPPhy> RadioApplicationClient for VLPSocket<P> {
    type Error = VLPError;

    async fn run<
        'a,
        'b,
        R: RawMutex,
        TXP: RadioApplicationPackage,
        RXP: RadioApplicationPackage,
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        radio_tx: Receiver<'a, R, TXP, TXN>,
        radio_rx: Sender<'b, R, RXP, RXN>,
    ) -> ! {
        loop {
            match self.prio {
                Priority::Driver => {
                    if let Ok(tx_package) = radio_tx.try_recv() {
                        self.transmit(tx_package.encode()).await;
                    } else {
                        self.handoff().await;
                    }
                }
                Priority::Listener => {
                    if let Ok(Some(data)) = self.receive().await {
                        if let Some(decoded) = RXP::decode(data) {
                            radio_rx.send(decoded).await;
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
    // currently unused
    SoftArming(bool),
}

impl RadioApplicationPackage for ApplicationLayerRxPackage {
    fn encode(self) -> Vec<u8, 222> {
        let mut buffer = Vec::<u8, 222>::new();
        unsafe {
            for _ in 0..core::mem::size_of::<<ApplicationLayerRxPackage as Archive>::Archived>() {
                buffer.push_unchecked(0);
            }
        }
        let mut serializer = BufferSerializer::new(buffer);
        serializer.serialize_value(&self).unwrap();
        serializer.into_inner()
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerRxPackage> {
        if let Ok(archived) = check_archived_root::<ApplicationLayerRxPackage>(package.as_slice()) {
            let d: ApplicationLayerRxPackage = archived.deserialize(&mut rkyv::Infallible).unwrap();
            Some(d)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, defmt::Format)]

pub enum ApplicationLayerTxPackage {
    Telemetry(TelemetryData),
}

impl RadioApplicationPackage for ApplicationLayerTxPackage {
    fn encode(self) -> Vec<u8, 222> {
        match self {
            ApplicationLayerTxPackage::Telemetry(telemetry) => {
                let mut buffer = Vec::<u8, 222>::new();
                unsafe {
                    for _ in 0..core::mem::size_of::<<TelemetryData as Archive>::Archived>() {
                        buffer.push_unchecked(0);
                    }
                }
                let mut serializer = BufferSerializer::new(buffer);
                serializer.serialize_value(&telemetry).unwrap();
                serializer.into_inner()
            }
        }
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerTxPackage> {
        if let Ok(archived) = check_archived_root::<TelemetryData>(package.as_slice()) {
            let d: TelemetryData = archived.deserialize(&mut rkyv::Infallible).unwrap();
            Some(ApplicationLayerTxPackage::Telemetry(d))
        } else {
            None
        }
    }
}
