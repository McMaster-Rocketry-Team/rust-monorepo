pub trait Clock: Copy {
    fn now_ms(&self) -> f64;
}
