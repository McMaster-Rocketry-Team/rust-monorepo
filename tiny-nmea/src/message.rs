use heapless::String;

#[derive(Debug, Clone)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u8,
}

#[derive(Debug)]
pub enum NMEAMessage {
    GLL {
        talker: String<2>,
        latitude: f32,
        longitude: f32,
        utc: Time,
    },
    GSV {
        talker: String<2>,
        satellites_visible: u8,
    },
}
