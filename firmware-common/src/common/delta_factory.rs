use either::Either;

pub trait Deltable: Sized + Clone {
    type DeltaType;

    fn add_delta(&self, delta: &Self::DeltaType) -> Option<Self>;

    fn subtract(&self, other: &Self) -> Option<Self::DeltaType>;
}

pub struct DeltaFactory<T: Deltable>
{
    last_value: Option<T>,
}

impl<T: Deltable> DeltaFactory<T>
{
    pub fn new() -> Self {
        Self {
            last_value: None,
        }
    }

    pub fn push(&mut self, value: T) -> Either<T, T::DeltaType> {
        if let Some(last_value) = self.last_value.take() {
            if let Some(delta) = value.subtract(&last_value) {
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

#[cfg(test)]
mod test {
    use crate::{driver::adc::ADCData, Volt};

    use super::*;

    #[test]
    fn test_delta_factory() {
        let a = ADCData::<Volt>::new(20.0);
        let b = ADCData::<Volt>::new(23.0);

        let mut factory = DeltaFactory::<ADCData::<Volt>>::new();
        let a_out = factory.push(a);
        let b_out = factory.push(b);
        println!("{:?}", a_out);
        println!("{:?}", b_out);
    }
}