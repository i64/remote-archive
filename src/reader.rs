pub trait NReader {
    fn read_u8(&mut self) -> std::io::Result<u8>;
    fn read_u16(&mut self) -> std::io::Result<u16>;
    fn read_u32(&mut self) -> std::io::Result<u32>;
    fn read_u64(&mut self) -> std::io::Result<u64>;
    fn read_str(&mut self, size: usize) -> std::io::Result<String>;
    fn read_bytes(&mut self, size: usize) -> std::io::Result<&[u8]>;
    fn read_arr<const N: usize>(&mut self, size: usize) -> std::io::Result<[u8; N]>;
    fn __read(&mut self, size: usize) -> std::io::Result<&[u8]>;
}
