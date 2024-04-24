use crate::{Event, Master, RequestError};
use core::cell::RefCell;
use core::{future::poll_fn, task::Poll};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::signal::Signal;
use embedded_hal_async::delay::DelayNs;
use firmware_common::driver::clock::Clock;
use firmware_common::driver::{
    arming::HardwareArming as ArmingDriver, camera::Camera as CameraCtrlDriver, gps::NmeaSentence,
    gps::GPS as GPSDriver, pyro::Continuity as ContinuityDriver, pyro::PyroCtrl as PyroCtrlDriver,
    serial::Serial,
};
use heapless::String;

pub struct MasterPyroCtrl<'a, S: Serial, D: DelayNs + Copy> {
    master: &'a Master<S, D>,
    pyro_channel: u8,
}

impl<'a, S: Serial, D: DelayNs + Copy> MasterPyroCtrl<'a, S, D> {
    pub fn new(master: &'a Master<S, D>, pyro_channel: u8) -> Self {
        Self {
            master,
            pyro_channel,
        }
    }
}

impl<'a, S: Serial, D: DelayNs + Copy> PyroCtrlDriver for MasterPyroCtrl<'a, S, D> {
    type Error = RequestError<S::Error>;

    async fn set_enable(&mut self, enable: bool) -> Result<(), RequestError<S::Error>> {
        self.master.pyro_ctrl(self.pyro_channel, enable).await?;
        Ok(())
    }
}

pub struct MasterPyroContinuity<'a, S: Serial, D: DelayNs + Copy> {
    master: &'a Master<S, D>,
    pyro_channel: u8,
    signal: Signal<NoopRawMutex, bool>,
    continuity: BlockingMutex<NoopRawMutex, RefCell<bool>>,
}

impl<'a, S: Serial, D: DelayNs + Copy> MasterPyroContinuity<'a, S, D> {
    pub fn new(master: &'a Master<S, D>, pyro_channel: u8) -> Self {
        Self {
            master,
            pyro_channel,
            signal: Signal::new(),
            continuity: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub async fn run(&self) -> ! {
        match self.master.get_continuity(self.pyro_channel).await {
            Ok(continuity) => {
                self.continuity.lock(|c| *c.borrow_mut() = continuity);
                self.signal.signal(continuity);
            }
            Err(e) => {
                defmt::error!("Error getting initial continuity: {:?}", e);
            }
        }
        poll_fn::<(), _>(|cx| {
            self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();

                if let Some(Event::Continuity {
                    pyro_channel,
                    continuity,
                }) = *last_event
                    && pyro_channel == self.pyro_channel
                {
                    last_event.take();
                    self.continuity.lock(|c| *c.borrow_mut() = continuity);
                    self.signal.signal(continuity);
                }
            });

            self.master.register_waker(cx.waker());
            Poll::Pending
        })
        .await;
        defmt::unreachable!()
    }
}

impl<'a, S: Serial, D: DelayNs + Copy> ContinuityDriver for &MasterPyroContinuity<'a, S, D> {
    type Error = ();

    async fn wait_continuity_change(&mut self) -> Result<bool, ()> {
        Ok(self.signal.wait().await)
    }

    async fn read_continuity(&mut self) -> Result<bool, ()> {
        Ok(self.continuity.lock(|c| *c.borrow_mut()))
    }
}

pub struct MasterHardwareArming<'a, S: Serial, D: DelayNs + Copy> {
    master: &'a Master<S, D>,
    signal: Signal<NoopRawMutex, bool>,
    armed: BlockingMutex<NoopRawMutex, RefCell<bool>>,
}

impl<'a, S: Serial, D: DelayNs + Copy> MasterHardwareArming<'a, S, D> {
    pub fn new(master: &'a Master<S, D>) -> Self {
        Self {
            master,
            signal: Signal::new(),
            armed: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub async fn run(&self) -> ! {
        match self.master.get_hardware_arming().await {
            Ok(armed) => {
                self.armed.lock(|c| *c.borrow_mut() = armed);
                self.signal.signal(armed);
            }
            Err(e) => {
                defmt::error!("Error getting initial hardware arming: {:?}", e);
            }
        }
        poll_fn::<(), _>(|cx| {
            self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();

                if let Some(Event::HardwareArming { armed }) = *last_event {
                    last_event.take();
                    self.armed.lock(|a| *a.borrow_mut() = armed);
                    self.signal.signal(armed);
                }
            });

            self.master.register_waker(cx.waker());
            Poll::Pending
        })
        .await;
        defmt::unreachable!()
    }
}

impl<'a, S: Serial, D: DelayNs + Copy> ArmingDriver for &MasterHardwareArming<'a, S, D> {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, ()> {
        Ok(self.signal.wait().await)
    }

    async fn read_arming(&mut self) -> Result<bool, ()> {
        Ok(self.armed.lock(|a| *a.borrow_mut()))
    }
}

pub struct MasterGPS<'a, S: Serial, D: DelayNs + Copy, K: Clock> {
    master: &'a Master<S, D>,
    signal: Signal<NoopRawMutex, ()>,
    sentence: BlockingMutex<NoopRawMutex, RefCell<Option<NmeaSentence>>>,
    clock: K,
}

impl<'a, S: Serial, D: DelayNs + Copy, K: Clock> MasterGPS<'a, S, D, K> {
    pub fn new(master: &'a Master<S, D>, clock: K) -> Self {
        Self {
            master,
            signal: Signal::new(),
            sentence: BlockingMutex::new(RefCell::new(None)),
            clock,
        }
    }

    pub async fn run(&self) -> ! {
        poll_fn::<(), _>(|cx| {
            self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();
                if let Some(Event::NmeaSentence { sentence, length }) = *last_event {
                    let nmea_sentence = NmeaSentence {
                        sentence: String::from_iter(
                            sentence
                                .into_iter()
                                .take(length as usize)
                                .map(|c| c as char),
                        ),
                        timestamp: self.clock.now_ms(),
                    };
                    last_event.take();
                    self.sentence
                        .lock(|s| *s.borrow_mut() = Some(nmea_sentence));
                    self.signal.signal(());
                }
            });

            self.master.register_waker(cx.waker());
            Poll::Pending
        })
        .await;
        defmt::unreachable!()
    }
}

impl<'a, S: Serial, D: DelayNs + Copy, K: Clock> GPSDriver for &MasterGPS<'a, S, D, K> {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        loop {
            self.signal.wait().await;
            let sentence = self.sentence.lock(|s| s.borrow_mut().take());
            if let Some(sentence) = sentence {
                return sentence;
            }
        }
    }
}

pub struct MasterCameraCtrl<'a, S: Serial, D: DelayNs + Copy> {
    master: &'a Master<S, D>,
}

impl<'a, S: Serial, D: DelayNs + Copy> MasterCameraCtrl<'a, S, D> {
    pub fn new(master: &'a Master<S, D>) -> Self {
        Self { master }
    }
}

impl<'a, S: Serial, D: DelayNs + Copy> CameraCtrlDriver for MasterCameraCtrl<'a, S, D> {
    type Error = RequestError<S::Error>;

    async fn set_recording(&mut self, is_recording: bool) -> Result<(), RequestError<S::Error>> {
        self.master.camera_ctrl(is_recording).await?;
        Ok(())
    }
}
