use async_trait::async_trait;

use std::{
    convert::From,
    io::{Read, Seek},
    ops::Range,
};

use reqwest::{self};

const PARTIAL_CONTENT: u16 = 206;

const CONTENT_RANGE: &str = "Content-Range";
const CONTENT_TYPE: &str = "Content-Type";
const RANGE: &str = "Range";

#[derive(Clone)]
pub struct RemoteFile {
    client: reqwest::Client,
    url: String,
    content_type: SupportedTypes,
    pub size: usize,
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
    pub async fn try_new(client: reqwest::Client, url: &str) -> Option<Self> {
        let (content_type, total_len) = check_range(&client, url).await?;
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
    async fn do_range_request(
        &mut self,
        range: Range<usize>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        dbg!(&range);
        self.client
            .get(&self.url)
            .header(RANGE, &format!("bytes={}-{}", range.start, range.end))
            .send()
            .await
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let response = self
            .do_range_request(self.offset..self.offset + buf.len())
            .await
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::Other))?;

        let response_data = response
            .bytes()
            .await
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::Other))?;

        let response_size = response_data.len() - 1;

        assert!(buf.len() >= response_size);

        // idk why it appends '\n'
        buf.copy_from_slice(&response_data[..(response_size)]);

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

pub async fn check_range(client: &reqwest::Client, url: &str) -> Option<(SupportedTypes, usize)> {
    if let Ok(resp) = do_range_request(client, url, 0..1).await {
        if resp.status() == PARTIAL_CONTENT {
            let content_range = resp.headers().get(CONTENT_RANGE)?.to_str().ok()?;
            let content_type = resp.headers().get(CONTENT_TYPE)?.to_str().ok()?;

            let mut splited_range = content_range.split('/');
            splited_range.next();
            let total_len = str::parse::<usize>(splited_range.next()?).ok()?;

            return Some((content_type.into(), total_len));
        }
        return None;
    }
    None
}

async fn do_range_request(
    client: &reqwest::Client,
    url: &str,
    range: Range<usize>,
) -> Result<reqwest::Response, reqwest::Error> {
    client
        .get(url)
        .header(RANGE, &format!("bytes={}-{}", range.start, range.end))
        .send()
        .await
}
