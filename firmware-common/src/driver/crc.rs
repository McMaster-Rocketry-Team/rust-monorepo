pub trait Crc {
    fn reset(&mut self);

    fn feed(&mut self, word: u32);

    fn read(&self) -> u32;

    fn calculate(&mut self, data: &[u8]) -> u32 {
        self.reset();
        let words = data.len() / 4;
        for i in 0..words {
            let value = u32::from_be_bytes(data[(i * 4)..((i + 1) * 4)].try_into().unwrap());
            self.feed(value);
        }

        let remaining_length = data.len() % 4;
        let mut last_word = [0u8; 4];
        for i in 0..remaining_length {
            last_word[i] = data[(words * 4) + i];
        }
        let value = u32::from_be_bytes(last_word);
        self.feed(value);

        self.read()
    }
}
