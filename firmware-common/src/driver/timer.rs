pub trait Timer: Copy {
    async fn sleep(&self, ms: u64);
    fn now_mills(&self) -> u64;
    fn now_micros(&self) -> u64;
}
