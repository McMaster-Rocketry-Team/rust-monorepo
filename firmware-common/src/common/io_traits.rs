pub trait AsyncReader {
    async fn read_slice(&mut self, length: usize) -> &[u8];

    async fn read_u8(&mut self) -> u8 {
        self.read_slice(1).await[0]
    }

    async fn read_u16(&mut self) -> u16 {
        u16::from_be_bytes(self.read_slice(2).await.try_into().unwrap())
    }

    async fn read_u32(&mut self) -> u32 {
        u32::from_be_bytes(self.read_slice(4).await.try_into().unwrap())
    }

    async fn read_u64(&mut self) -> u64 {
        u64::from_be_bytes(self.read_slice(8).await.try_into().unwrap())
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