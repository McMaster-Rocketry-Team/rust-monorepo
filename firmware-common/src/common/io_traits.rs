pub trait AsyncReader {
    async fn read_slice<'b>(&mut self, buffer: &'b mut [u8], length: usize) -> &'b [u8];

    async fn read_u8(&mut self, buffer: &mut [u8]) -> u8 {
        self.read_slice(buffer, 1).await[0]
    }

    async fn read_u16(&mut self, buffer: &mut [u8]) -> u16 {
        u16::from_be_bytes(self.read_slice(buffer, 2).await.try_into().unwrap())
    }

    async fn read_u32(&mut self, buffer: &mut [u8]) -> u32 {
        u32::from_be_bytes(self.read_slice(buffer, 4).await.try_into().unwrap())
    }

    async fn read_u64(&mut self, buffer: &mut [u8]) -> u64 {
        u64::from_be_bytes(self.read_slice(buffer, 8).await.try_into().unwrap())
    }
}

pub trait AsyncWriter {
    async fn extend_from_slice(&mut self, slice: &[u8]);

    async fn extend_from_u8(&mut self, value: u8) {
        self.extend_from_slice(&[value]).await;
    }

    async fn extend_from_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_be_bytes()).await;
    }

    async fn extend_from_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_be_bytes()).await;
    }

    async fn extend_from_u64(&mut self, value: u64) {
        self.extend_from_slice(&value.to_be_bytes()).await;
    }
}

pub trait Writer {
    fn extend_from_slice(&mut self, slice: &[u8]);

    fn extend_from_u8(&mut self, value: u8) {
        self.extend_from_slice(&[value]);
    }

    fn extend_from_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_be_bytes());
    }

    fn extend_from_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_be_bytes());
    }

    fn extend_from_u64(&mut self, value: u64) {
        self.extend_from_slice(&value.to_be_bytes());
    }
}
