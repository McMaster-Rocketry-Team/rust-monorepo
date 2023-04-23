pub trait Serial {
    async fn write(&mut self, data: &[u8]) -> Result<(), ()>;
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ()>;

    async fn writeln(&mut self, data: &[u8]) -> Result<(), ()>{
        self.write(data).await?;
        self.write(b"\r\n").await?;
        Ok(())
    }
}
