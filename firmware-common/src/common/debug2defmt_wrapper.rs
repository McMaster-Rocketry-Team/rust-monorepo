use core::fmt;

#[derive(Debug, defmt::Format)]
pub struct Debug2DefmtWrapper<T: fmt::Debug>(#[defmt(Debug2Format)] pub T);
