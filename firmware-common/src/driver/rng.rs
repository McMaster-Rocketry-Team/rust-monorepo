pub trait RNG {
    async fn next_bytes(&mut self, buffer: &mut [u8]);

    async fn next_u64(&mut self) -> u64 {
        let mut buffer = [0u8; 8];
        self.next_bytes(&mut buffer).await;
        u64::from_be_bytes(buffer)
    }

    async fn next_u32(&mut self) -> u32 {
        let mut buffer = [0u8; 4];
        self.next_bytes(&mut buffer).await;
        u32::from_be_bytes(buffer)
    }

    async fn next_u16(&mut self) -> u16 {
        let mut buffer = [0u8; 2];
        self.next_bytes(&mut buffer).await;
        u16::from_be_bytes(buffer)
    }

    async fn next_byte(&mut self) -> u8 {
        let mut buffer = [0u8; 1];
        self.next_bytes(&mut buffer).await;
        buffer[0]
    }
}

pub struct DummyRNG {}

impl RNG for DummyRNG {
    async fn next_bytes(&mut self, _buffer: &mut [u8]) {}
}
