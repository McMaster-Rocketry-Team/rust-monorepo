use embassy_sync::{
    blocking_mutex::raw::NoopRawMutex,
    channel::{Receiver, Sender},
};

use crate::{
    driver::{radio::RadioReceiveInfo, timer::Timer},
    vlp::application_layer::{
            ApplicationLayerRxPackage, ApplicationLayerTxPackage, RadioApplicationPackage,
        }, RadioPhy,
};

pub struct PVLP<T: RadioPhy>(pub T);

impl<T: RadioPhy> PVLP<T> {
    async fn tx<P: RadioApplicationPackage>(&mut self, package: P) {
        self.0.tx(&package.encode()).await;
    }

    async fn rx<P: RadioApplicationPackage>(&mut self) -> Option<(RadioReceiveInfo, Option<P>)> {
        self.0
            .rx()
            .await
            .map(|(info, data)| (info, P::decode(data)))
            .ok()
    }

    async fn rx_with_timeout<P: RadioApplicationPackage>(
        &mut self,
        timeout_ms: u32,
    ) -> Option<(RadioReceiveInfo, Option<P>)> {
        let result = self.0
            .rx_with_timeout(timeout_ms)
            .await;

        match result {
            Ok(Some((info, data))) =>Some((info, P::decode(data))),
            Ok(None) => None,
            Err(e) => {
                log_warn!("rx_with_timeout error: {:?}", e);
                None
            },
        }
    }

    fn set_frequency(&mut self, freq: u32) {
        self.0.set_frequency(freq);
    }
}

const FREQ_LIST: [u32; 8] = [
    902300000, 903900000, 905500000, 907100000, 908700000, 910300000, 911900000, 913500000,
];

pub struct PVLPMaster<Y: RadioPhy, T: Timer> {
    phy: PVLP<Y>,
    timer: T,
    start_time: Option<f64>,
}

impl<Y: RadioPhy, T: Timer> PVLPMaster<Y, T> {
    pub fn new(phy: PVLP<Y>, timer: T) -> Self {
        Self {
            phy,
            timer,
            start_time: None,
        }
    }

    fn get_frequency(&mut self) -> u32 {
        if self.start_time.is_none() {
            self.start_time = Some(self.timer.now_mills());
        }
        let time = self.timer.now_mills() - self.start_time.unwrap();
        let index = (time / 17000.0) as usize;
        let index = index % FREQ_LIST.len();
        FREQ_LIST[index]
    }

    pub async fn tx_and_rx(
        &mut self,
        package: ApplicationLayerTxPackage,
    ) -> Option<ApplicationLayerRxPackage> {
        let freq = self.get_frequency();
        log_info!("tx on freq: {}", freq);
        self.phy.set_frequency(freq);
        self.phy.tx(package).await;
        if let Some((_, Some(rx_package))) = self
            .phy
            .rx_with_timeout::<ApplicationLayerRxPackage>(1500)
            .await
        {
            Some(rx_package)
        } else {
            None
        }
    }
}

pub struct PVLPSlave<'a, 'b, Y: RadioPhy, T: Timer, const N: usize, const M: usize> {
    phy: PVLP<Y>,
    timer: T,
    master_start_time: Option<f64>,
    rx: Sender<'a, NoopRawMutex, (RadioReceiveInfo, ApplicationLayerTxPackage), N>,
    tx: Receiver<'b, NoopRawMutex, ApplicationLayerRxPackage, M>,
}

impl<'a, 'b, Y: RadioPhy, T: Timer, const N: usize, const M: usize> PVLPSlave<'a, 'b, Y, T, N, M> {
    pub fn new(
        phy: PVLP<Y>,
        timer: T,
        rx: Sender<'a, NoopRawMutex, (RadioReceiveInfo, ApplicationLayerTxPackage), N>,
        tx: Receiver<'b, NoopRawMutex, ApplicationLayerRxPackage, M>,
    ) -> Self {
        Self {
            phy,
            timer,
            master_start_time: None,
            rx,
            tx,
        }
    }

    // returns the current frequency, and the time (ms) until the next frequency change
    fn get_frequency(&self) -> (u32, u32) {
        if let Some(master_start_time) = self.master_start_time {
            let time = self.timer.now_mills() - master_start_time;
            let index = (time / 17000.0) as usize;
            let index = index % FREQ_LIST.len();
            let time_until_next = 17000 - (time % 17000.0) as u32;
            (FREQ_LIST[index], time_until_next)
        } else {
            (FREQ_LIST[0], 0xFFFFF)
        }
    }

    fn calculate_airtime(&self, len: u8) -> f64 {
        return (len as f64 * 8.0) / 980.0 * 1000.0;
    }

    pub async fn run(&mut self) -> ! {
        loop {
            let (freq, timeout) = self.get_frequency();
            log_info!("rx on freq: {} for {}s", freq, timeout as f64 / 1000.0);
            self.phy.set_frequency(freq);
            let rx = self
                .phy
                .rx_with_timeout::<ApplicationLayerTxPackage>(timeout)
                .await;
            if let Some((info, Some(package))) = rx {
                log_info!("{:?} {:?}", info, package);
                if self.master_start_time.is_none() {
                    log_info!("master start time set");
                    self.master_start_time =
                        Some(self.timer.now_mills() - self.calculate_airtime(info.len) - 50.0);
                }
                self.rx.try_send((info, package)).ok();
                if let Ok(tx_package) = self.tx.try_receive() {
                    self.phy.tx(tx_package).await;
                }
            } else {
                log_info!("rx timeout {:?}", rx);
            }
        }
    }
}
