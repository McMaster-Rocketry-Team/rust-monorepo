use crate::driver::crc::Crc;

pub struct U8Crc<'a, C>
where
    C: Crc,
{
    crc: &'a mut C,
    crc_buffer: [u8; 4],
    crc_buffer_index: usize,
}

impl<'a, C> U8Crc<'a, C>
where
    C: Crc,
{
    pub fn new(crc: &'a mut C) -> Self {
        crc.reset();
        Self {
            crc,
            crc_buffer: [0; 4],
            crc_buffer_index: 0,
        }
    }

    pub fn read_crc(&self) -> u32 {
        self.crc.read()
    }

    pub fn process(&mut self, data: &[u8]) {
        let mut crc_feed_offset = if self.crc_buffer_index != 0 {
            4 - self.crc_buffer_index
        } else {
            0
        };

        let length = data.len();
        if length < crc_feed_offset {
            (&mut self.crc_buffer[self.crc_buffer_index..(self.crc_buffer_index + length)])
                .copy_from_slice(data);
            self.crc_buffer_index += length;
        } else {
            if crc_feed_offset != 0 {
                (&mut self.crc_buffer[self.crc_buffer_index..])
                    .copy_from_slice(&data[..crc_feed_offset]);
                self.crc.feed(u32::from_be_bytes(self.crc_buffer));
                self.crc_buffer_index = 0;
            }

            while length - crc_feed_offset >= 4 {
                self.crc.feed(u32::from_be_bytes(
                    (&data[crc_feed_offset..(crc_feed_offset + 4)])
                        .try_into()
                        .unwrap(),
                ));
                crc_feed_offset += 4;
            }

            if length - crc_feed_offset > 0 {
                (&mut self.crc_buffer[..(length - crc_feed_offset)])
                    .copy_from_slice(&data[crc_feed_offset..]);
                self.crc_buffer_index = length - crc_feed_offset;
            }
        }
    }
}
