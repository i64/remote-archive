use std::ops::Range;

pub mod zip;

pub struct Entry<T> {
    entry: T,
    range: Range<u64>,
}

impl<T> Entry<T> {
    pub fn new(entry: T, start_pos: u64, end_pos: u64) -> Self {
        Self {
            entry,
            range: start_pos..end_pos,
        }
    }
}
pub trait FileType {
    type EntryType;

    fn read_entry(&mut self) -> std::io::Result<Entry<Self::EntryType>>;
    fn start_from(&mut self, start: usize) -> std::io::Result<u64>;
}
