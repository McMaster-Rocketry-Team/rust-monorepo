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
use crate::{common::telemetry::telemetry_data::TelemetryData, driver::timer::Timer};
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
        T: Timer,
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        timer: T,
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
        T: Timer,
        const TXN: usize,
        const RXN: usize,
    >(
        &mut self,
        timer: T,
        radio_tx: Receiver<'a, R, TXP, TXN>,
        radio_rx: Sender<'b, R, RXP, RXN>,
    ) -> ! {
        loop {
            timer.sleep(100.0).await;
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
    Synced,
}

impl RadioApplicationPackage for ApplicationLayerRxPackage {
    fn encode(self) -> Vec<u8, 222> {
        let mut buffer = Vec::<u8, 222>::new();
        unsafe {
            buffer.push_unchecked(0x69);
            for _ in 0..core::mem::size_of::<<ApplicationLayerRxPackage as Archive>::Archived>() {
                buffer.push_unchecked(0);
            }
        }
        let mut serializer = BufferSerializer::new(buffer.split_at_mut(1).1);
        serializer.serialize_value(&self).unwrap();
        buffer
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerRxPackage> {
        if package.len() == 0 || package[0] != 0x69 {
            return None;
        }
        if let Ok(archived) =
            check_archived_root::<ApplicationLayerRxPackage>(&package.as_slice()[1..])
        {
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
                    buffer.push_unchecked(0x69);
                    for _ in 0..core::mem::size_of::<<TelemetryData as Archive>::Archived>() {
                        buffer.push_unchecked(0);
                    }
                }
                let mut serializer = BufferSerializer::new(buffer.split_at_mut(1).1);
                serializer.serialize_value(&telemetry).unwrap();
                buffer
            }
        }
    }

    fn decode(package: Vec<u8, 222>) -> Option<ApplicationLayerTxPackage> {
        if package.len() == 0 || package[0] != 0x69 {
            return None;
        }
        if let Ok(archived) = check_archived_root::<TelemetryData>(&package.as_slice()[1..]) {
            let d: TelemetryData = archived.deserialize(&mut rkyv::Infallible).unwrap();
            Some(ApplicationLayerTxPackage::Telemetry(d))
        } else {
            None
        }
    }
}
