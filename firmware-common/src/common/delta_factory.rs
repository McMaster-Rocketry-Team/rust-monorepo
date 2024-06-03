use core::ops::Sub;

use either::Either;

pub trait Deltable: Sized + Clone {
    type DeltaType;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self>;
}

pub struct DeltaFactory<T>
where
    T: Deltable,
    for<'a> &'a T: Sub<Output = Option<T::DeltaType>>,
{
    last_value: Option<T>,
}

impl<T> DeltaFactory<T>
where
    T: Deltable,
    for<'a> &'a T: Sub<Output = Option<T::DeltaType>>,
{
    pub fn new() -> Self {
        Self {
            last_value: None,
        }
    }

    pub fn push(&mut self, value: T) -> Either<T, T::DeltaType> {
        if let Some(last_value) = self.last_value.take() {
            if let Some(delta) = &value - &last_value {
                self.last_value = Some(last_value.add_delta(&delta).unwrap());
                return Either::Right(delta);
            }
        }

        self.last_value = Some(value.clone());
        Either::Left(value)
    }
}

pub struct UnDeltaFactory<T>
where
    T: Deltable,
{
    last_value: Option<T>,
}

impl<T> UnDeltaFactory<T>
where
    T: Deltable,
{
    pub fn new() -> Self {
        Self {
            last_value: None,
        }
    }

    pub fn push(&mut self, value: T) -> T {
        self.last_value = Some(value.clone());
        value
    }

    pub fn push_delta(&mut self, delta: T::DeltaType) -> Option<T> {
        if let Some(last_value) = self.last_value.take() {
            if let Some(new_value) = last_value.add_delta(&delta) {
                self.last_value = Some(new_value.clone());
                return Some(new_value);
            }
        }
        None
    }
}
