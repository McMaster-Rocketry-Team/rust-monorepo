use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use futures::join;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    claim_devices,
    common::{
        console::{
            console::Console,
            console_program::{start_console_program_2, ConsoleProgram},
        },
        device_manager::prelude::*,
    },
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, timer::Timer},
    vlp::{
        application_layer::{
            ApplicationLayerRxPackage, ApplicationLayerTxPackage, RadioApplicationClient,
        },
        VLPSocket,
    },
};

#[inline(never)]
pub async fn gcm_main<const N: usize, const M: usize>(
    _fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
    console1: &Console<impl Serial, N>,
    console2: &Console<impl Serial, N>,
) {
    claim_devices!(device_manager, vlp_phy);

    vlp_phy.set_output_power(22);

    let radio_tx = Channel::<NoopRawMutex, ApplicationLayerRxPackage, 1>::new();
    let radio_rx = Channel::<NoopRawMutex, ApplicationLayerTxPackage, 3>::new();

    let vert_calibration_prog_fut = start_console_program_2(
        device_manager,
        console1,
        console2,
        GCMVerticalCalibration {
            sender: radio_tx.sender(),
        },
    );
    let clear_storage_prog_fut = start_console_program_2(
        device_manager,
        console1,
        console2,
        GCMClearStorage {
            sender: radio_tx.sender(),
        },
    );
    let get_telemetry_prog_fut = start_console_program_2(
        device_manager,
        console1,
        console2,
        GCMGetTelemetry::new(radio_rx.receiver()),
    );

    let radio_fut = async {
        let mut vlp_socket = VLPSocket::await_establish(vlp_phy).await.unwrap();
        vlp_socket.run(radio_tx.receiver(), radio_rx.sender()).await;
    };

    #[allow(unreachable_code)]
    {
        join!(
            vert_calibration_prog_fut,
            clear_storage_prog_fut,
            get_telemetry_prog_fut,
            radio_fut
        );
    }
}

#[derive(Clone)]
struct GCMVerticalCalibration<'a, const N: usize> {
    sender: Sender<'a, NoopRawMutex, ApplicationLayerRxPackage, N>,
}

impl<'a, const N: usize> ConsoleProgram for GCMVerticalCalibration<'a, N> {
    fn id(&self) -> u64 {
        0x10
    }

    async fn run(&mut self, _serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        self.sender
            .send(ApplicationLayerRxPackage::VerticalCalibration)
            .await;
    }
}

#[derive(Clone)]
struct GCMClearStorage<'a, const N: usize> {
    sender: Sender<'a, NoopRawMutex, ApplicationLayerRxPackage, N>,
}

impl<'a, const N: usize> ConsoleProgram for GCMClearStorage<'a, N> {
    fn id(&self) -> u64 {
        0x11
    }

    async fn run(&mut self, _serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        self.sender
            .send(ApplicationLayerRxPackage::ClearStorage)
            .await;
    }
}

#[derive(Clone)]
struct GCMGetTelemetry<'a, const N: usize> {
    receiver: Receiver<'a, NoopRawMutex, ApplicationLayerTxPackage, N>,
    buffer: [u8; 512],
}

impl<'a, const N: usize> GCMGetTelemetry<'a, N> {
    fn new(receiver: Receiver<'a, NoopRawMutex, ApplicationLayerTxPackage, N>) -> Self {
        Self {
            receiver,
            buffer: [0; 512],
        }
    }
}

impl<'a, const N: usize> ConsoleProgram for GCMGetTelemetry<'a, N> {
    fn id(&self) -> u64 {
        0x12
    }

    async fn run(&mut self, serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        if let Ok(package) = self.receiver.try_recv() {
            match package {
                ApplicationLayerTxPackage::Telemetry(telemetry) => {
                    log_info!("Telemetry: {:#?}", telemetry);
                    let json_len = serde_json_core::to_slice(&telemetry, &mut self.buffer).unwrap();
                    let json = &self.buffer[..json_len];
                    serial.write(json).await;
                }
            }
        }
    }
}
