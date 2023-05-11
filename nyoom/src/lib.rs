#![cfg_attr(not(test), no_std)]

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    pipe::{Reader, Writer},
};

// Nyoom V3 Encoder
pub struct NyoomEncoder {}

impl NyoomEncoder {
    fn new() -> Self {
        todo!()
    }

    // Function to encode a handshake message.
    // The `handshake` parameter contains all the handshake information except the rocket model
    // The `rocket_model_reader` allows this function to read the model byte-by-byte,
    //   instead of reading the entire model at once, which will overwhelme the RAM.
    // The encoded message should be written to `output_writer`.
    async fn encodeHandshake<'r, 'w, MR, const NR: usize, MW, const NW: usize>(
        &mut self,
        handshake: (), // TODO
        rocket_model_reader: Reader<'r, MR, NR>,
        output_writer: Writer<'w, MW, NW>,
    ) where
        MR: RawMutex,
        MW: RawMutex,
    {
        todo!()
    }

    // Function to encode a data message.
    // The `data parameter contains all the information to be encoded
    // The encoded message should be written to `output_writer`.
    fn encodeData<'r, 'w, MW, const NW: usize>(
        &mut self,
        data: (), // TODO
        output_writer: Writer<'w, MW, NW>,
    ) where
        MW: RawMutex,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[futures_test::test]
    async fn it_works() {
        todo!()
    }
}
