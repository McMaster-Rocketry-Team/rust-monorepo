use rkyv::{Archive, Deserialize, Serialize};

use crate::common::delta_logger::delta_factory::Deltable;


#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct AvionicsState {
    pub timestamp: f64,
    pub armed: bool,
    pub soft_armed: bool,
}

impl AvionicsState {
    pub fn eq_except_timestamp(&self, other: &Self) -> bool {
        self.armed == other.armed && self.soft_armed == other.soft_armed
    }
}

#[derive(defmt::Format, Debug, Clone, Archive, Deserialize, Serialize)]
pub struct AvionicsStateDelta {
    pub timestamp: u16,
}

mod factories {
    use crate::fixed_point_factory;

    fixed_point_factory!(Timestamp, 0.0, 1100.0, f64, u16);
}

impl Deltable for AvionicsState {
    type DeltaType = AvionicsStateDelta;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self> {
        Some(Self {
            timestamp: self.timestamp + factories::Timestamp::to_float(delta.timestamp),
            armed: self.armed,
            soft_armed: self.soft_armed,
        })
    }

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType> {
        if self.eq_except_timestamp(other) {
            Some(AvionicsStateDelta {
                timestamp: factories::Timestamp::to_fixed_point(self.timestamp - other.timestamp)?,
            })
        } else {
            None
        }
    }
}
