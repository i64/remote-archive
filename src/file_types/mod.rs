use std::ops::RangeInclusive;

pub mod zip;

#[derive(Debug)]
pub struct Entry {
    pub filename: String,
    // pos: RangeInclusive<usize>,
}

pub struct EntryIter<'a, R> {
    source: &'a mut R,
    count: usize,
}

pub trait FileType: Sized {
    fn entry_iter(&mut self) -> std::io::Result<EntryIter<Self>>;
}
