use std::{
    convert::From,
    io::{Read, Seek},
    ops::Range,
};

use exact_reader::File;
use ureq;

const PARTIAL_CONTENT: u16 = 206;

const CONTENT_RANGE: &str = "Content-Range";
const CONTENT_TYPE: &str = "Content-Type";
const RANGE: &str = "Range";

pub struct RemoteAdapter {
    client: ureq::Agent,
    url: String,
    size: usize,
    pos: usize,
}

impl RemoteAdapter {
    pub fn try_new(client: ureq::Agent, url: &str) -> Option<Self> {
        let total_len = check_range(&client, url)?;
        Some(Self {
            client,
            size: total_len,
            pos: 0,
            url: url.to_string(),
        })
    }

    #[inline]
    pub(super) fn do_range_request(
        &mut self,
        range: Range<usize>,
    ) -> Result<ureq::Response, ureq::Error> {
        do_range_request(&self.client, &self.url, range)
    }
}

impl Read for RemoteAdapter {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.pos;
        let end = {
            let _end = self.pos + buf.len();
            match _end > self.size {
                true => self.size,
                false => _end,
            }
        };

        assert!(self.size >= end);
        let response = self.do_range_request(start..end).unwrap();

        let mut vec = Vec::with_capacity(buf.len());

        let response_size = response
            .into_reader()
            .take(buf.len() as u64)
            .read_to_end(&mut vec)?;

        buf[..vec.len()].copy_from_slice(&vec);

        self.pos += response_size;

        Ok(response_size)
    }
}

impl Seek for RemoteAdapter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let (base_pos, offset) = match pos {
            std::io::SeekFrom::Start(o) => (0, o as i64),
            std::io::SeekFrom::End(o) => (self.size as i64, o),
            std::io::SeekFrom::Current(o) => (self.pos as i64, o),
        };

        let new_pos = base_pos + offset;

        if new_pos.is_negative() {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }

        self.pos = new_pos as usize;
        Ok(self.pos as u64)
    }
}

impl From<RemoteAdapter> for File<RemoteAdapter> {
    fn from(value: RemoteAdapter) -> Self {
        let size = value.size;
        let filename = value.url.clone();
        let file = value;
        Self {
            file,
            size,
            filename,
        }
    }
}

fn check_range(client: &ureq::Agent, url: &str) -> Option<usize> {
    let resp = do_range_request(client, url, 0..1).ok()?;

    if resp.status() == PARTIAL_CONTENT {
        let content_range = resp.header(CONTENT_RANGE)?;

        let mut splited_range = content_range.split('/');
        splited_range.next();
        let total_len = str::parse::<usize>(splited_range.next()?).ok()?;

        return Some(total_len);
    }
    None
}

fn do_range_request(
    client: &ureq::Agent,
    url: &str,
    range: Range<usize>,
) -> Result<ureq::Response, ureq::Error> {
    dbg!(&range);
    client
        .get(url)
        .set(RANGE, &format!("bytes={}-{}", range.start, range.end))
        .call()
}
