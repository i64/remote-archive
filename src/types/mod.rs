use async_trait::async_trait;
use std::ops::Range;
pub mod zip;

pub struct Entry<T> {
    pub entry: T,
    pub range: Range<u64>,
}

impl<T> Entry<T> {
    pub fn new(entry: T, start_pos: u64, end_pos: u64) -> Self {
        Self {
            entry,
            range: start_pos..end_pos,
        }
    }
}

#[async_trait]
pub trait FileType {
    type EntryType;

    async fn read_entry(&mut self) -> std::io::Result<Entry<Self::EntryType>>;
    async fn start_from(&mut self, start: usize, haystack_size: usize) -> std::io::Result<u64>;
}
