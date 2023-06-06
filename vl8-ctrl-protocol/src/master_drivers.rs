use crate::{Event, Master, RequestError};
use core::cell::RefCell;
use core::{future::poll_fn, task::Poll};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use firmware_common::driver::{
    arming::HardwareArming as ArmingDriver, gps::NmeaSentence, gps::GPS as GPSDriver,
    pyro::Continuity as ContinuityDriver, pyro::PyroCtrl as PyroCtrlDriver, serial::Serial,
    timer::Timer,
};
use heapless::String;

pub struct MasterPyroCtrl<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    pyro_channel: u8,
}

impl<'a, S: Serial, T: Timer> MasterPyroCtrl<'a, S, T> {
    pub fn new(master: &'a Master<S, T>, pyro_channel: u8) -> Self {
        Self {
            master,
            pyro_channel,
        }
    }
}

impl<'a, S: Serial, T: Timer> PyroCtrlDriver for MasterPyroCtrl<'a, S, T> {
    type Error = RequestError<S::Error>;

    async fn set_enable(&mut self, enable: bool) -> Result<(), RequestError<S::Error>> {
        self.master.pyro_ctrl(self.pyro_channel, enable).await?;
        Ok(())
    }
}

pub struct MasterPyroContinuity<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    pyro_channel: u8,
    signal: Signal<CriticalSectionRawMutex, bool>,
    continuity: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl<'a, S: Serial, T: Timer> MasterPyroContinuity<'a, S, T> {
    pub fn new(master: &'a Master<S, T>, pyro_channel: u8) -> Self {
        Self {
            master,
            pyro_channel,
            signal: Signal::new(),
            continuity: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub async fn run(&self) -> ! {
        poll_fn::<(), _>(|cx| {
            self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();

                if let Some(Event::Continuity {
                    pyro_channel,
                    continuity,
                }) = *last_event && pyro_channel == self.pyro_channel {
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

impl<'a, S: Serial, T: Timer> ContinuityDriver for &MasterPyroContinuity<'a, S, T> {
    type Error = ();

    async fn wait_continuity_change(&mut self) -> Result<bool, ()> {
        Ok(self.signal.wait().await)
    }

    async fn read_continuity(&mut self) -> Result<bool, ()> {
        Ok(self.continuity.lock(|c| *c.borrow_mut()))
    }
}

pub struct MasterHardwareArming<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    signal: Signal<CriticalSectionRawMutex, bool>,
    armed: BlockingMutex<CriticalSectionRawMutex, RefCell<bool>>,
}

impl<'a, S: Serial, T: Timer> MasterHardwareArming<'a, S, T> {
    pub fn new(master: &'a Master<S, T>) -> Self {
        Self {
            master,
            signal: Signal::new(),
            armed: BlockingMutex::new(RefCell::new(false)),
        }
    }

    pub async fn run(&self) -> ! {
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

impl<'a, S: Serial, T: Timer> ArmingDriver for &MasterHardwareArming<'a, S, T> {
    type Error = ();

    async fn wait_arming_change(&mut self) -> Result<bool, ()> {
        Ok(self.signal.wait().await)
    }

    async fn read_arming(&mut self) -> Result<bool, ()> {
        Ok(self.armed.lock(|a| *a.borrow_mut()))
    }
}

pub struct MasterGPS<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    signal: Signal<CriticalSectionRawMutex, ()>,
    sentence: BlockingMutex<CriticalSectionRawMutex, RefCell<Option<NmeaSentence>>>,
    timer: T,
}

impl<'a, S: Serial, T: Timer> MasterGPS<'a, S, T> {
    pub fn new(master: &'a Master<S, T>, timer: T) -> Self {
        Self {
            master,
            signal: Signal::new(),
            sentence: BlockingMutex::new(RefCell::new(None)),
            timer,
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
                        timestamp: self.timer.now_mills(),
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

impl<'a, S: Serial, T: Timer> GPSDriver for &MasterGPS<'a, S, T> {
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
