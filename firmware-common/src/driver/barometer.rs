use super::timer::Timer;

#[derive(defmt::Format, Debug)]
pub struct BaroReading {
    pub timestamp: u64,   // ms
    pub temperature: f32, // C
    pub pressure: f32,    // Pa
}

pub trait Barometer {
    type Error;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn read(&mut self) -> Result<BaroReading, Self::Error>;
}

pub struct DummyBarometer<T: Timer> {
    timer: T,
}

impl<T: Timer> DummyBarometer<T> {
    pub fn new(timer: T) -> Self {
        Self { timer }
    }
}
impl<T: Timer> Barometer for DummyBarometer<T> {
    type Error = ();

    async fn reset(&mut self) -> Result<(), ()> {
        Ok(())
    }

    async fn read(&mut self) -> Result<BaroReading, ()> {
        self.timer.sleep(1).await;
        Ok(BaroReading {
            timestamp: 0,
            temperature: 0.0,
            pressure: 0.0,
        })
    }
}
