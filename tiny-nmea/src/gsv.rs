use crate::message::NMEAMessage;
use crate::message::NMEAMessage::GSV;
use crate::substring;
use heapless::String;
use heapless::Vec;

pub fn parse_gsv(fields: Vec<&str, 41>) -> Result<NMEAMessage, ()> {
    let talker = substring!(fields[1], 0, 2);
    let satellites_visible = fields[4].parse::<u8>().map_err(|_| ())?;
    Ok(GSV {
        talker,
        satellites_visible,
    })
}
