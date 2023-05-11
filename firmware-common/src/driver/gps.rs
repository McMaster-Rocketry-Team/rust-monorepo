use heapless::String;

pub struct NmeaSentence{
    pub sentence: String<84>,
    pub timestamp: u64,
}

pub trait GPS {
    async fn reset(&mut self);

    async fn set_enable(&mut self, enable: bool);

    fn read_next_nmea_sentence(&mut self) -> Option<NmeaSentence>;
}
