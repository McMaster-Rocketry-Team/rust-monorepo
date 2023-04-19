use core::marker::PhantomData;

use crate::driver::serial::Serial;

use super::program::ConsoleProgram;

pub struct Hello<T: Serial> {
    phantom: PhantomData<T>,
}

impl<T: Serial> Hello<T> {
    pub const fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: Serial> ConsoleProgram<T> for Hello<T> {
    fn name(&self) -> &'static str {
        "hello"
    }

    async fn start(&self, serial: &mut T) -> Result<(), ()> {
        serial.write(b"Hello, world!\r\n").await?;

        Ok(())
    }
}
