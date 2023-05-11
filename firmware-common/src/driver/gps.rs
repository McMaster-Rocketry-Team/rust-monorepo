use tiny_nmea::NMEA;

pub trait GPS {
    async fn reset(&mut self);

    async fn set_enable(&mut self, enable: bool);

    async fn receive(&mut self) -> NMEA;
}
