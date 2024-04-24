use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Channel, Receiver, Sender},
};
use futures::join;
use vlfs::{Crc, Flash, VLFS};

use crate::{
    allocator::HEAP,
    claim_devices,
    common::{
        console::{
            console::Console,
            console_program::{start_console_program_2, ConsoleProgram},
        },
        device_manager::prelude::*,
        pvlp::{PVLPSlave, PVLP},
    },
    device_manager_type,
    driver::{gps::GPS, indicator::Indicator, radio::RadioReceiveInfo},
    vlp::application_layer::{ApplicationLayerRxPackage, ApplicationLayerTxPackage},
};

#[inline(never)]
pub async fn gcm_main<const N: usize, const M: usize>(
    _fs: &VLFS<impl Flash, impl Crc>,
    device_manager: device_manager_type!(),
    console1: &Console<impl Serial, N>,
    console2: &Console<impl Serial, N>,
) {
    // Init 1KiB heap
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 1024;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    claim_devices!(device_manager, radio_phy);

    radio_phy.set_output_power(22);

    let radio_tx = Channel::<NoopRawMutex, ApplicationLayerRxPackage, 1>::new();
    let radio_rx = Channel::<NoopRawMutex, (RadioReceiveInfo, ApplicationLayerTxPackage), 3>::new();

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
    let softarm_prog_fut = start_console_program_2(
        device_manager,
        console1,
        console2,
        GCMSoftArm {
            sender: radio_tx.sender(),
        },
    );
    let softdearm_prog_fut = start_console_program_2(
        device_manager,
        console1,
        console2,
        GCMSoftDearm {
            sender: radio_tx.sender(),
        },
    );

    let radio_fut = async {
        let mut socket = PVLPSlave::new(
            PVLP(radio_phy),
            device_manager.clock,
            radio_rx.sender(),
            radio_tx.receiver(),
        );
        socket.run().await;
    };

    #[allow(unreachable_code)]
    {
        join!(
            vert_calibration_prog_fut,
            clear_storage_prog_fut,
            get_telemetry_prog_fut,
            radio_fut,
            softarm_prog_fut,
            softdearm_prog_fut,
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
struct GCMSoftArm<'a, const N: usize> {
    sender: Sender<'a, NoopRawMutex, ApplicationLayerRxPackage, N>,
}

impl<'a, const N: usize> ConsoleProgram for GCMSoftArm<'a, N> {
    fn id(&self) -> u64 {
        0x13
    }

    async fn run(&mut self, _serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        self.sender
            .send(ApplicationLayerRxPackage::SoftArming(true))
            .await;
    }
}

#[derive(Clone)]
struct GCMSoftDearm<'a, const N: usize> {
    sender: Sender<'a, NoopRawMutex, ApplicationLayerRxPackage, N>,
}

impl<'a, const N: usize> ConsoleProgram for GCMSoftDearm<'a, N> {
    fn id(&self) -> u64 {
        0x14
    }

    async fn run(&mut self, _serial: &mut impl Serial, _device_manager: device_manager_type!()) {
        self.sender
            .send(ApplicationLayerRxPackage::SoftArming(false))
            .await;
    }
}

#[derive(Clone)]
struct GCMGetTelemetry<'a, const N: usize> {
    receiver: Receiver<'a, NoopRawMutex, (RadioReceiveInfo, ApplicationLayerTxPackage), N>,
    buffer: [u8; 512],
}

impl<'a, const N: usize> GCMGetTelemetry<'a, N> {
    fn new(
        receiver: Receiver<'a, NoopRawMutex, (RadioReceiveInfo, ApplicationLayerTxPackage), N>,
    ) -> Self {
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
        if let Ok((info, package)) = self.receiver.try_receive() {
            match package {
                ApplicationLayerTxPackage::Telemetry(telemetry) => {
                    log_info!("Telemetry: {:?} {:?}", info, telemetry);
                    let json_len = serde_json_core::to_slice(&info, &mut self.buffer).unwrap();
                    let json = &self.buffer[..json_len];
                    serial.write(json).await;
                    serial.write(b"|").await;
                    let json_len = serde_json_core::to_slice(&telemetry, &mut self.buffer).unwrap();
                    let json = &self.buffer[..json_len];
                    serial.write(json).await;
                }
            }
        } else {
            serial.write(b"x").await;
        }
    }
}
