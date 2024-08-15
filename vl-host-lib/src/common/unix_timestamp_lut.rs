use map_range::MapRange;

pub struct UnixTimestampLUT {
    // (boot_timestamp, unix_timestamp)
    points: Vec<(f64, f64)>,
}

impl UnixTimestampLUT {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    pub fn add_timestamp(&mut self, boot_timestamp: f64, unix_timestamp: f64) {
        self.points.push((boot_timestamp, unix_timestamp));
    }

    pub fn sort_timestamps(&mut self) {
        self.points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn get_unix_timestamp(&self, boot_timestamp: f64) -> Option<f64> {
        if self.points.len() < 2 {
            return None;
        }

        let binary_search_result = self.points.binary_search_by(|(point_boot_timestamp, _)| {
            point_boot_timestamp.partial_cmp(&boot_timestamp).unwrap()
        });

        match binary_search_result {
            Ok(i) => {
                return Some(self.points[i].1);
            }
            Err(i) => {
                let i = i.max(1).min(self.points.len() - 1);
                let before_point = self.points[i - 1];
                let after_point = self.points[i];
                return Some(
                    boot_timestamp
                        .map_range(before_point.0..after_point.0, before_point.1..after_point.1),
                );
            }
        }
    }
}
