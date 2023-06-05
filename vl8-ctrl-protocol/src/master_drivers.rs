use crate::{Event, Master, RequestError};
use core::{future::poll_fn, task::Poll};
use defmt::warn;
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
    continuity: bool,
}

impl<'a, S: Serial, T: Timer> MasterPyroContinuity<'a, S, T> {
    pub fn new(master: &'a Master<S, T>, pyro_channel: u8) -> Self {
        Self {
            master,
            pyro_channel,
            continuity: false,
        }
    }
}

impl<'a, S: Serial, T: Timer> ContinuityDriver for MasterPyroContinuity<'a, S, T> {
    type Error = ();

    async fn wait_continuity_change(&mut self) {
        poll_fn(|cx| {
            let result = self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();

                if let Some(Event::Continuity {
                    pyro_channel,
                    continuity,
                }) = *last_event
                &&
                pyro_channel == self.pyro_channel
                {
                        self.continuity = continuity;
                        last_event.take();
                        return Poll::Ready(());
                }

                return Poll::Pending;
            });

            if let Poll::Pending = result {
                self.master.wakers_reg.lock(|reg| {
                    if let Err(_) = reg.borrow_mut().register(cx.waker()) {
                        warn!("Failed to register waker");
                    }
                });
            }

            result
        })
        .await;
    }

    async fn read_continuity(&mut self) -> Result<bool, ()> {
        Ok(self.continuity)
    }
}

pub struct MasterHarwareArming<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    armed: bool,
}

impl<'a, S: Serial, T: Timer> MasterHarwareArming<'a, S, T> {
    pub fn new(master: &'a Master<S, T>) -> Self {
        Self {
            master,
            armed: false,
        }
    }
}

impl<'a, S: Serial, T: Timer> ArmingDriver for MasterHarwareArming<'a, S, T> {
    async fn wait_arming_change(&mut self) {
        poll_fn(|cx| {
            let result = self.master.last_event.lock(|last_event| {
                let mut last_event = last_event.borrow_mut();

                if let Some(Event::HardwareArming { armed }) = *last_event {
                    self.armed = armed;
                    last_event.take();
                    return Poll::Ready(());
                }

                return Poll::Pending;
            });

            if let Poll::Pending = result {
                self.master.wakers_reg.lock(|reg| {
                    if let Err(_) = reg.borrow_mut().register(cx.waker()) {
                        warn!("Failed to register waker");
                    }
                });
            }

            result
        })
        .await;
    }

    async fn read_arming(&mut self) -> bool {
        self.armed
    }
}

pub struct MasterGPS<'a, S: Serial, T: Timer> {
    master: &'a Master<S, T>,
    timer: T,
}

impl<'a, S: Serial, T: Timer> MasterGPS<'a, S, T> {
    pub fn new(master: &'a Master<S, T>, timer: T) -> Self {
        Self { master, timer }
    }
}

impl<'a, S: Serial, T: Timer> GPSDriver for MasterGPS<'a, S, T> {
    async fn next_nmea_sentence(&mut self) -> NmeaSentence {
        poll_fn(|cx| {
            let result = self.master.last_event.lock(|last_event| {
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
                    return Poll::Ready(nmea_sentence);
                }

                return Poll::Pending;
            });

            if let Poll::Pending = result {
                self.master.wakers_reg.lock(|reg| {
                    if let Err(_) = reg.borrow_mut().register(cx.waker()) {
                        warn!("Failed to register waker");
                    }
                });
            }

            result
        })
        .await
    }
}
