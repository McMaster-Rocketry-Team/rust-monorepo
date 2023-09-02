use crate::Crc;

pub struct DummyCrc {}

impl Crc for DummyCrc {
    fn reset(&mut self) {}

    fn feed(&mut self, _word: u32) {}

    fn read(&self) -> u32 {
        0x69696969
    }
}
