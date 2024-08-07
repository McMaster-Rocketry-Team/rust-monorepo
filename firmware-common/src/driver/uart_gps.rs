use crate::{common::sensor_reading::SensorReading, Clock};

use super::{
    gps::{GPSData, GPS},
    timestamp::BootTimestamp,
};
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use embedded_io_async::{Read, Error};
use heapless::String;
use nmea::Nmea;

pub struct UARTGPS {
    last_location: Signal<NoopRawMutex, SensorReading<BootTimestamp, GPSData>>,
}

impl UARTGPS {
    pub fn new() -> Self {
        Self {
            last_location: Signal::new(),
        }
    }

    pub async fn run(&self, rx: &mut impl Read, clock: impl Clock) -> ! {
        let mut buffer = [0; 9];
        let mut sentence = String::<84>::new();
        let mut nmea = Nmea::default();
        loop {
            match rx.read(&mut buffer).await {
                Ok(length) => {
                    for i in 0..length {
                        sentence.push(buffer[i] as char).ok();

                        if buffer[i] == 10u8 || sentence.len() == 84 {
                            if sentence.as_bytes()[0] != b'$' {
                                sentence.clear();
                            }

                            nmea.parse(sentence.as_str()).ok();

                            self.last_location
                                .signal(SensorReading::new(clock.now_ms(), (&nmea).into()));

                            sentence.clear();
                            for j in (i + 1)..length {
                                sentence.push(buffer[j] as char).ok();
                            }
                        }
                    }
                }
                Err(e) => {
                    log_error!("Error reading from UART: {:?}", e.kind());
                    yield_now().await;
                }
            }
        }
    }
}

impl GPS for &UARTGPS {
    type Error = ();

    async fn next_location(
        &mut self,
    ) -> Result<SensorReading<BootTimestamp, GPSData>, Self::Error> {
        Ok(self.last_location.wait().await)
    }
}
