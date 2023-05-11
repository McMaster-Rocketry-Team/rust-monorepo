pub trait AsyncReader {
    type Error;
    type ReadStatus;

    async fn read_slice<'b>(
        &mut self,
        buffer: &'b mut [u8],
        length: usize,
    ) -> Result<(&'b [u8], Self::ReadStatus), Self::Error>;

    async fn read_all<'b>(
        &mut self,
        read_buffer: &'b mut [u8],
    ) -> Result<(&'b [u8], Self::ReadStatus), Self::Error> {
        self.read_slice(read_buffer, read_buffer.len()).await
    }

    async fn read_u8(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(Option<u8>, Self::ReadStatus), Self::Error> {
        self.read_slice(buffer, 1).await.map(|(buffer, status)| {
            if buffer.len() == 1 {
                (Some(buffer[0]), status)
            } else {
                (None, status)
            }
        })
    }

    async fn read_u16(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(Option<u16>, Self::ReadStatus), Self::Error> {
        self.read_slice(buffer, 2).await.map(|(buffer, status)| {
            if buffer.len() == 2 {
                (Some(u16::from_be_bytes(buffer.try_into().unwrap())), status)
            } else {
                (None, status)
            }
        })
    }

    async fn read_u32(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(Option<u32>, Self::ReadStatus), Self::Error> {
        self.read_slice(buffer, 4).await.map(|(buffer, status)| {
            if buffer.len() == 4 {
                (Some(u32::from_be_bytes(buffer.try_into().unwrap())), status)
            } else {
                (None, status)
            }
        })
    }

    async fn read_u64(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<(Option<u64>, Self::ReadStatus), Self::Error> {
        self.read_slice(buffer, 8).await.map(|(buffer, status)| {
            if buffer.len() == 8 {
                (Some(u64::from_be_bytes(buffer.try_into().unwrap())), status)
            } else {
                (None, status)
            }
        })
    }
}

pub trait AsyncWriter {
    type Error;

    async fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), Self::Error>;

    async fn extend_from_u8(&mut self, value: u8) -> Result<(), Self::Error> {
        self.extend_from_slice(&[value]).await
    }

    async fn extend_from_u16(&mut self, value: u16) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes()).await
    }

    async fn extend_from_u32(&mut self, value: u32) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes()).await
    }

    async fn extend_from_u64(&mut self, value: u64) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes()).await
    }
}

pub trait Writer {
    type Error;

    fn extend_from_slice(&mut self, slice: &[u8]) -> Result<(), Self::Error>;

    fn extend_from_u8(&mut self, value: u8) -> Result<(), Self::Error> {
        self.extend_from_slice(&[value])
    }

    fn extend_from_u16(&mut self, value: u16) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes())
    }

    fn extend_from_u32(&mut self, value: u32) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes())
    }

    fn extend_from_u64(&mut self, value: u64) -> Result<(), Self::Error> {
        self.extend_from_slice(&value.to_be_bytes())
    }
}
