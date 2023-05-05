pub trait AsyncReader {
    type Error;

    async fn read_slice<'b>(
        &mut self,
        buffer: &'b mut [u8],
        length: usize,
    ) -> Result<&'b [u8], Self::Error>;

    async fn read_u8(&mut self, buffer: &mut [u8]) -> Result<u8, Self::Error> {
        Ok(self.read_slice(buffer, 1).await?[0])
    }

    async fn read_u16(&mut self, buffer: &mut [u8]) -> Result<u16, Self::Error> {
        Ok(u16::from_be_bytes(
            self.read_slice(buffer, 2).await?.try_into().unwrap(),
        ))
    }

    async fn read_u32(&mut self, buffer: &mut [u8]) -> Result<u32, Self::Error> {
        Ok(u32::from_be_bytes(
            self.read_slice(buffer, 4).await?.try_into().unwrap(),
        ))
    }

    async fn read_u64(&mut self, buffer: &mut [u8]) -> Result<u64, Self::Error> {
        Ok(u64::from_be_bytes(
            self.read_slice(buffer, 8).await?.try_into().unwrap(),
        ))
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
