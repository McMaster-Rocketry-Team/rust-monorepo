#![cfg_attr(not(test), no_std)]

use embassy_sync::{
    blocking_mutex::raw::RawMutex,
    pipe::{Reader, Writer},
};

pub struct NyoomEncoder {}

impl NyoomEncoder {
    fn new() -> Self {
        todo!()
    }

    async fn addHandshake<'r, 'w, MR, const NR: usize, MW, const NW: usize>(
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

    fn addData<'r, 'w, MW, const NW: usize>(
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
