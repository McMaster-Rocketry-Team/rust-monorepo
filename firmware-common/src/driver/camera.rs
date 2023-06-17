pub trait Camera {
    type Error: defmt::Format;

    async fn set_recording(&mut self, is_recording: bool) -> Result<(), Self::Error>;
}

pub struct DummyCamera {}

impl Camera for DummyCamera {
    type Error = ();

    async fn set_recording(&mut self, _is_recording: bool) -> Result<(), ()> {
        Ok(())
    }
}
