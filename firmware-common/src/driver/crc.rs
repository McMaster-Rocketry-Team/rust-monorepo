use core::ops::{DerefMut, Deref};

use defmt::info;
use embassy_sync::{mutex::MutexGuard, blocking_mutex::raw::RawMutex};

pub trait Crc {
    fn reset(&mut self);

    fn feed(&mut self, word: u32);

    fn read(&self) -> u32;

    // TODO remove
    fn calculate(&mut self, data: &[u8]) -> u32 {
        self.reset();
        info!("CRC calculate: {}", data);
        let words = data.len() / 4;
        for i in 0..words {
            let value = u32::from_be_bytes(data[(i * 4)..((i + 1) * 4)].try_into().unwrap());
            self.feed(value);
        }

        let remaining_length = data.len() % 4;
        if remaining_length > 0 {
            let mut last_word = [0xFFu8; 4];
            for i in 0..remaining_length {
                last_word[i] = data[(words * 4) + i];
            }
            let value = u32::from_be_bytes(last_word);
            self.feed(value);
        }

        self.read()
    }
}

impl<'a, M, T> Crc for MutexGuard<'a, M, T>
where
    M: RawMutex,
    T: Crc,
{
    fn reset(&mut self) {
        self.deref_mut().reset();
    }

    fn feed(&mut self, word: u32) {
        self.deref_mut().feed(word);
    }

    fn read(&self) -> u32 {
        self.deref().read()
    }
}