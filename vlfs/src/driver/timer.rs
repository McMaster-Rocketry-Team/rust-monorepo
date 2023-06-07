pub trait Timer: Copy {
    async fn sleep(&self, ms: f64);
    fn now_mills(&self) -> f64;
}