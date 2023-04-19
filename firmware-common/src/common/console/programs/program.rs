use crate::driver::serial::Serial;

pub trait ConsoleProgram<T: Serial> {
    fn name(&self) -> &'static str;
    async fn start(&self, serial: &mut T) -> Result<(), ()>;
}
