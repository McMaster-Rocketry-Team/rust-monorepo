pub trait Timer: Clone {
    fn now_ms(&self) -> f64;
}
