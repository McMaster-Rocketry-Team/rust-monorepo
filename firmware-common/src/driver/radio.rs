use core::ops::DerefMut;
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::MutexGuard};
use heapless::Vec;
use serde::Serialize as SerdeSerialize;

#[derive(SerdeSerialize, Debug, defmt::Format)]
pub struct RadioReceiveInfo {
    pub rssi: i16,
    pub snr: i16,
    pub len: u8,
}

const MAX_PAYLOAD_LENGTH: usize = 222;
pub trait RadioPhy {
    type Error: defmt::Format;

    async fn reset(&mut self) -> Result<(), Self::Error>;
    async fn tx(&mut self, payload: &[u8]);
    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), Self::Error>;
    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<Option<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>)>, Self::Error>;

    fn set_frequency(&mut self, frequency: u32);
    fn increment_frequency(&mut self);
    fn reset_frequency(&mut self);
    fn set_output_power(&mut self, power: i32);
}

impl<'a, M, T> RadioPhy for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: RadioPhy,
{
    type Error = T::Error;

    async fn reset(&mut self) -> Result<(), Self::Error> {
        self.deref_mut().reset().await
    }

    async fn tx(&mut self, payload: &[u8]) {
        self.deref_mut().tx(payload).await
    }

    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), Self::Error> {
        self.deref_mut().rx().await
    }

    async fn rx_with_timeout(
        &mut self,
        timeout_ms: u32,
    ) -> Result<Option<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>)>, Self::Error> {
        self.deref_mut().rx_with_timeout(timeout_ms).await
    }

    fn increment_frequency(&mut self) {
        self.deref_mut().increment_frequency()
    }

    fn set_frequency(&mut self, frequency: u32) {
        self.deref_mut().set_frequency(frequency)
    }

    fn reset_frequency(&mut self) {
        self.deref_mut().reset_frequency()
    }

    fn set_output_power(&mut self, power: i32) {
        self.deref_mut().set_output_power(power)
    }
}

pub struct DummyRadio {}

impl RadioPhy for DummyRadio {
    type Error = ();

    async fn reset(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn tx(&mut self, _payload: &[u8]) {}

    async fn rx(&mut self) -> Result<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>), Self::Error> {
        Ok((
            RadioReceiveInfo {
                rssi: 0,
                snr: 0,
                len: 0,
            },
            Vec::new(),
        ))
    }

    async fn rx_with_timeout(
        &mut self,
        _timeout_ms: u32,
    ) -> Result<Option<(RadioReceiveInfo, Vec<u8, MAX_PAYLOAD_LENGTH>)>, Self::Error> {
        Ok(Some((
            RadioReceiveInfo {
                rssi: 0,
                snr: 0,
                len: 0,
            },
            Vec::new(),
        )))
    }

    fn set_frequency(&mut self, _frequency: u32) {}

    fn increment_frequency(&mut self) {}

    fn reset_frequency(&mut self) {}

    fn set_output_power(&mut self, _power: i32) {}
}
