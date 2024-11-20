use core::future::Future;

pub trait SendSpawner {
    fn spawn<F>(&self, future: impl FnOnce() -> F + Send)
    where
        F: Future<Output = !> + 'static;
}
