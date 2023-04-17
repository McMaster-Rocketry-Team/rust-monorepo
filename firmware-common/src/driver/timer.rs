pub trait Timer: Copy {
    async fn sleep(&self, ms: u64);
    async fn now(&self) -> u64;
}
