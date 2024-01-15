use core::fmt::Debug;

pub(crate) struct DebugBytes<'a>(pub &'a [u8]);

impl defmt::Format for DebugBytes<'_> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "[");
        let mut iter = self.0.iter().peekable();
        while let Some(byte) = iter.next() {
            defmt::write!(fmt, "{=u8:02X}", byte);
            if iter.peek().is_some() {
                defmt::write!(fmt, ", ");
            }
        }
        defmt::write!(fmt, "]");
    }
}

impl Debug for DebugBytes<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("[")?;
        let mut iter = self.0.iter().peekable();
        while let Some(byte) = iter.next() {
            write!(f, "{:02X}", byte)?;
            if iter.peek().is_some() {
                write!(f, ", ")?;
            }
        }
        f.write_str("]")
    }
}
