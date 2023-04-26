use super::*;

impl<F, C> VLFS<F, C>
where
    F: SpiFlash,
    C: Crc,
{
    pub async fn open_file_for_write(&self, file_id: u64) -> Option<FileWriter<F, C>> {
        Some(FileWriter::new(self))
    }
}

pub struct FileWriter<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    vlfs: &'a VLFS<F, C>,
    buffer1: [u8; 256 + 5],
    buffer2: [u8; 256 + 5],
    is_using_buffer1: bool,
}

impl<'a, F, C> FileWriter<'a, F, C>
where
    F: SpiFlash,
    C: Crc,
{
    fn new(vlfs: &'a VLFS<F, C>) -> Self {
        FileWriter {
            vlfs,
            buffer1: [0u8; 256 + 5],
            buffer2: [0u8; 256 + 5],
            is_using_buffer1: true,
        }
    }

    pub async fn awa(&self) {}
}
