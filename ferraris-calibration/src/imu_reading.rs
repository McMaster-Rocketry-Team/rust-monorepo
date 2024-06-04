pub trait IMUReadingTrait: Clone {
    fn timestamp(&self) -> f64;
    fn acc(&self) -> [f32; 3];
    fn gyro(&self) -> [f32; 3];
    fn set_acc(&mut self, acc: [f32; 3]);
    fn set_gyro(&mut self, gyro: [f32; 3]);
}

#[derive(Debug, Clone)]
pub struct IMUReading {
    pub timestamp: f64, // ms
    pub acc: [f32; 3],  // m/s^2
    pub gyro: [f32; 3],
}


impl IMUReadingTrait for IMUReading {
    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    fn acc(&self) -> [f32; 3] {
        self.acc
    }

    fn gyro(&self) -> [f32; 3] {
        self.gyro
    }

    fn set_acc(&mut self, acc: [f32; 3]) {
        self.acc = acc;
    }

    fn set_gyro(&mut self, gyro: [f32; 3]) {
        self.gyro = gyro;
    }
}