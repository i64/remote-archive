use async_trait::async_trait;

#[async_trait]
pub trait NReader {
    async fn read_u8(&mut self) -> std::io::Result<u8>;
    async fn read_u16(&mut self) -> std::io::Result<u16>;
    async fn read_u32(&mut self) -> std::io::Result<u32>;
    async fn read_u64(&mut self) -> std::io::Result<u64>;
    async fn read_str(&mut self, size: usize) -> std::io::Result<String>;
    async fn read_bytes(&mut self, size: usize) -> std::io::Result<&[u8]>;
    async fn read_arr<const N: usize>(&mut self, size: usize) -> std::io::Result<[u8; N]>;
    async fn __read(&mut self, size: usize) -> std::io::Result<&[u8]>;
}
