#[derive(Debug, defmt::Format)]
pub struct EmbeddedErrorWrapper<E: embedded_io::Error>(#[defmt(Debug2Format)] pub E);

impl<E: embedded_io::Error> From<E> for EmbeddedErrorWrapper<E> {
    fn from(error: E) -> Self {
        Self(error)
    }
}
