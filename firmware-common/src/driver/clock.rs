use vlfs::Timer as VLFSTimer;

pub trait Clock: Clone {
    fn now_ms(&self) -> f64;
}

#[derive(Clone)]
pub(crate) struct VLFSTimerWrapper<T: Clock>(pub(crate) T);

impl<T: Clock> VLFSTimer for VLFSTimerWrapper<T> {
    fn now_ms(&self) -> f64 {
        self.0.now_ms()
    }
}