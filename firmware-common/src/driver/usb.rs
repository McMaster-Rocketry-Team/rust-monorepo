pub trait USB {
    async fn write_packet_64b(&mut self, data: &[u8]) -> Result<(), ()>;
    async fn read_packet(&mut self, buffer: &mut [u8]) -> Result<usize, ()>;
}
