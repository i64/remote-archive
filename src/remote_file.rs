use std::{
    convert::From,
    io::{Read, Seek},
    ops::Range,
};

const PARTIAL_CONTENT: u16 = 206;

const CONTENT_RANGE: &str = "Content-Range";
const CONTENT_TYPE: &str = "Content-Type";
const RANGE: &str = "Range";

pub struct RemoteFile {
    client: ureq::Agent,
    url: String,
    content_type: SupportedTypes,
    size: usize,
    offset: usize,
}

#[derive(Clone, Copy)]
pub enum SupportedTypes {
    Zip,
    Unsupported,
}

impl From<&str> for SupportedTypes {
    fn from(type_name: &str) -> Self {
        match type_name {
            "application/zip" => Self::Zip,
            _ => Self::Unsupported,
        }
    }
}

impl RemoteFile {
    pub fn try_new(client: ureq::Agent, url: &str) -> Option<Self> {
        let (content_type, total_len) = check_range(&client, url)?;
        Some(Self {
            client,
            content_type,
            size: total_len,
            offset: 0,
            url: url.to_string(),
        })
    }

    pub fn content_type(&self) -> SupportedTypes {
        self.content_type
    }
    fn do_range_request(&mut self, range: Range<usize>) -> Result<ureq::Response, ureq::Error> {
        dbg!(&range);
        self.client
            .get(&self.url)
            .set(RANGE, &format!("bytes={}-{}", range.start, range.end))
            .call()
    }
}

impl Read for RemoteFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let response = self
            .do_range_request(self.offset..self.offset + buf.len())
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::Other))?;

        let mut vec = Vec::with_capacity(buf.len());

        let response_size = response
            .into_reader()
            .take(buf.len() as u64)
            .read_to_end(&mut vec)?;

        buf.copy_from_slice(&vec);

        self.offset += response_size;

        Ok(response_size)
    }
}

impl Seek for RemoteFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Current(o) => {
                if self.offset as isize + o as isize >= self.size as isize {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = (self.offset as isize + o as isize) as usize;
            }
            std::io::SeekFrom::End(o) => {
                if o as isize >= self.size as isize {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = (self.size as isize - 1 - o as isize) as usize;
            }
            std::io::SeekFrom::Start(o) => {
                if o as usize >= self.size {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.offset = o as usize;
            }
        }
        Ok(self.offset as u64)
    }
}

pub fn check_range(client: &ureq::Agent, url: &str) -> Option<(SupportedTypes, usize)> {
    if let Ok(resp) = do_range_request(client, url, 0..1) {
        if resp.status() == PARTIAL_CONTENT {
            let content_range = resp.header(CONTENT_RANGE)?;
            let content_type = resp.header(CONTENT_TYPE)?;

            let mut splited_range = content_range.split('/');
            splited_range.next();
            let total_len = str::parse::<usize>(splited_range.next()?).ok()?;

            return Some((content_type.into(), total_len));
        }
        return None;
    }
    None
}

fn do_range_request(
    client: &ureq::Agent,
    url: &str,
    range: Range<usize>,
) -> Result<ureq::Response, ureq::Error> {
    client
        .get(url)
        .set(RANGE, &format!("bytes={}-{}", range.start, range.end))
        .call()
}
