use super::serial::SplitableSerial;

pub trait SplitableUSB: SplitableSerial {
    async fn wait_connection(&mut self);
}

// impl<D: DelayNs> USB for DummyUSB<D> {
//     type Error = ();

//     async fn write_64b(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
//         Ok(())
//     }

//     async fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, Self::Error> {
//         loop {
//             self.delay.delay_ms(1000).await;
//         }
//     }

//     async fn wait_connection(&mut self) {
//         loop {
//             self.delay.delay_ms(1000).await;
//         }
//     }
// }
