use async_trait::async_trait;

#[async_trait]
pub trait AsyncWrite {
    async fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
    async fn flush(&mut self) -> std::io::Result<()>;
}

#[async_trait]
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
}
