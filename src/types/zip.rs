use std::{
    fmt::Debug,
    io::{Read, Seek, SeekFrom},
};

use super::{Entry, FileType};
use crate::reader::NReader;
use crate::remote_file::RemoteFile;

const MAGIC_BYTES: &[u8; 2] = b"PK";

macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl std::convert::TryFrom<usize> for $name {
            type Error = ();

            fn try_from(v: usize) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as usize => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
    #[derive(Debug)]
    enum SectionType {
        CentralDirEntry = 0x0201,
        LocalFile = 0x0403,
        EndOfCentralDir = 0x0605,
        DataDescriptor = 0x0807,
    }
}

back_to_enum! {
    #[derive(Debug)]
    enum Compression {
        None = 0,
        Shrunk = 1,
        Reduced1 = 2,
        Reduced2 = 3,
        Reduced3 = 4,
        Reduced4 = 5,
        Imploded = 6,
        Deflated = 8,
        EnhancedDeflated = 9,
        PkwareDclImploded = 10,
        Bzip2 = 12,
        Lzma = 14,
        IBMTerse = 18,
        IBMLz77Z = 19,
        Ppmd = 98,
    }
}

#[derive(Debug)]
pub enum SectionBody {
    CentralDirEntry(CentralDirEntry),
    LocalFile(LocalFile),
    EndOfCentralDir(EndOfCentralDir),
    DataDescriptor(DataDescriptor),
}
#[allow(dead_code)]
#[derive(Debug)]
pub struct DataDescriptor {
    crc32: u32,
    len_body_compressed: u32,
    len_body_uncompressed: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct CentralDirEntry {
    version_made_by: u16,
    version_needed_to_extract: u16,
    flags: u16,
    compression_method: Compression,
    last_mod_file_time: u16,
    last_mod_file_date: u16,
    crc32: u32,
    len_body_compressed: u32,
    len_body_uncompressed: u32,
    len_filename: u16,
    len_extra: u16,
    len_comment: u16,
    disk_number_start: u16,
    int_file_attr: u16,
    ext_file_attr: u32,
    ofs_local_header: u32,
    filename: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct LocalFileHeader {
    version: u16,
    flags: u16,
    compression_method: Compression,
    file_mod_time: u16,
    file_mod_date: u16,
    crc32: u32,
    len_body_compressed: u32,
    len_body_uncompressed: u32,
    len_filename: u16,
    len_extra: u16,
    filename: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct LocalFile {
    header: LocalFileHeader,
    body: Option<()>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EndOfCentralDir {
    disk_of_end_of_central_dir: u16,
    disk_of_central_dir: u16,
    num_central_dir_entries_on_disk: u16,
    num_central_dir_entries_total: u16,
    len_central_dir: u32,
    ofs_central_dir: u32,
    len_comment: u16,
    comment: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PkSection {
    section_body: SectionBody,
}

pub struct ZipFile {
    buf_offset: usize,
    file: RemoteFile,
    buf: Vec<u8>,
}

impl ZipFile {
    pub fn new(f: RemoteFile) -> Self {
        Self {
            buf_offset: 0,
            file: f,
            buf: Vec::new(),
        }
    }

    fn ensure(&mut self, size: usize) -> std::io::Result<usize> {
        if self.buf.len() >= size {
            self.buf.fill(0);
        } else {
            self.buf.resize(size, 0);
            self.buf.fill(0);
        }
        self.buf_offset = 0;
        self.file.read(&mut self.buf[0..size])
    }

    fn read_section(&mut self) -> std::io::Result<SectionBody> {
        const STRUCT_SIZE: usize = 4;
        self.ensure(STRUCT_SIZE)?;

        let magic = &self.read_bytes(2)?[..2];
        if MAGIC_BYTES != magic {
            return Err(std::io::ErrorKind::Unsupported.into());
        }
        let section_type = SectionType::try_from(self.read_u16()? as usize)
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::InvalidData))?;

        let section_body = match section_type {
            SectionType::CentralDirEntry => SectionBody::CentralDirEntry(self.central_dir_entry()?),
            SectionType::LocalFile => SectionBody::LocalFile(self.local_file()?),
            SectionType::EndOfCentralDir => {
                SectionBody::EndOfCentralDir(self.end_of_central_dir()?)
            }
            SectionType::DataDescriptor => SectionBody::DataDescriptor(self.data_descriptor()?),
        };

        Ok(section_body)
    }

    fn data_descriptor(&mut self) -> std::io::Result<DataDescriptor> {
        const STRUCT_SIZE: usize = 3 * 4;
        self.ensure(STRUCT_SIZE)?;

        Ok(DataDescriptor {
            crc32: self.read_u32()?,
            len_body_compressed: self.read_u32()?,
            len_body_uncompressed: self.read_u32()?,
        })
    }
    fn central_dir_entry(&mut self) -> std::io::Result<CentralDirEntry> {
        const STRUCT_SIZE: usize = 2 * 6 + 4 * 3 + 2 * 5 + 4 * 2;
        self.ensure(STRUCT_SIZE)?;

        let version_made_by = self.read_u16()?;
        let version_needed_to_extract = self.read_u16()?;
        let flags = self.read_u16()?;
        let compression_method = Compression::try_from(self.read_u16()? as usize)
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::InvalidData))?;
        let last_mod_file_time = self.read_u16()?;
        let last_mod_file_date = self.read_u16()?;
        let crc32 = self.read_u32()?;
        let len_body_compressed = self.read_u32()?;
        let len_body_uncompressed = self.read_u32()?;
        let len_filename = self.read_u16()?;
        let len_extra = self.read_u16()?;
        let len_comment = self.read_u16()?;
        let disk_number_start = self.read_u16()?;
        let int_file_attr = self.read_u16()?;
        let ext_file_attr = self.read_u32()?;
        let ofs_local_header = self.read_u32()?;

        self.ensure(len_filename as usize)?;
        let filename = self.read_str(len_filename as usize)?;

        self.seek(std::io::SeekFrom::Current((len_extra + len_comment) as i64))?;

        Ok(CentralDirEntry {
            version_made_by,
            version_needed_to_extract,
            flags,
            compression_method,
            last_mod_file_time,
            last_mod_file_date,
            crc32,
            len_body_compressed,
            len_body_uncompressed,
            len_filename,
            len_extra,
            len_comment,
            disk_number_start,
            int_file_attr,
            ext_file_attr,
            ofs_local_header,
            filename,
        })
    }
    fn local_file(&mut self) -> std::io::Result<LocalFile> {
        let local_file_header = {
            const STRUCT_SIZE: usize = 2 * 5 + 4 * 3 + 2 * 2;
            self.ensure(STRUCT_SIZE)?;

            let version = self.read_u16()?;
            let flags = self.read_u16()?;
            let compression_method = Compression::try_from(self.read_u16()? as usize)
                .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::InvalidData))?;
            let file_mod_time = self.read_u16()?;
            let file_mod_date = self.read_u16()?;
            let crc32 = self.read_u32()?;
            let len_body_compressed = self.read_u32()?;
            let len_body_uncompressed = self.read_u32()?;
            let len_filename = self.read_u16()?;
            let len_extra = self.read_u16()?;

            self.ensure(len_filename as usize)?;
            let filename = self.read_str(len_filename as usize)?;

            LocalFileHeader {
                version,
                flags,
                compression_method,
                file_mod_time,
                file_mod_date,
                crc32,
                len_body_compressed,
                len_body_uncompressed,
                len_filename,
                len_extra,
                filename,
            }
        };

        self.seek(std::io::SeekFrom::Current(
            local_file_header.len_body_compressed as i64 + local_file_header.len_extra as i64,
        ))?;
        Ok(LocalFile {
            body: None,
            header: local_file_header,
        })
    }

    fn end_of_central_dir(&mut self) -> std::io::Result<EndOfCentralDir> {
        const STRUCT_SIZE: usize = 2 * 4 + 4 * 2 + 2 * 1;
        self.ensure(STRUCT_SIZE)?;

        let disk_of_end_of_central_dir = self.read_u16()?;
        let disk_of_central_dir = self.read_u16()?;
        let num_central_dir_entries_on_disk = self.read_u16()?;
        let num_central_dir_entries_total = self.read_u16()?;
        let len_central_dir = self.read_u32()?;
        let ofs_central_dir = self.read_u32()?;
        let len_comment = self.read_u16()?;

        self.ensure(len_comment as usize)?;
        let comment = self.read_str(len_comment as usize)?;

        Ok(EndOfCentralDir {
            disk_of_end_of_central_dir,
            disk_of_central_dir,
            num_central_dir_entries_on_disk,
            num_central_dir_entries_total,
            len_central_dir,
            ofs_central_dir,
            len_comment,
            comment,
        })
    }
}

impl FileType for ZipFile {
    type EntryType = SectionBody;

    fn read_entry(&mut self) -> std::io::Result<Entry<Self::EntryType>> {
        let start_pos = self.seek(SeekFrom::Current(0)).unwrap(); //get the current cursor
        let section = self.read_section()?;
        let end_pos = self.seek(SeekFrom::Current(0)).unwrap();

        Ok(Entry::new(section, start_pos, end_pos))
    }

    fn start_from(&mut self, start: usize) -> std::io::Result<u64> {
        const READ_SIZE: usize = 4096;
        const VALID_HEADERS: [&[u8]; 4] = [
            &0x02014b50u32.to_le_bytes(), // CentralDirectory
            &0x04034b50u32.to_le_bytes(), // LocalFileHeader
            &0x06054b50u32.to_le_bytes(), // EndOfCentralDirectory
            &0x08074b50u32.to_le_bytes(), // DataDescriptor
        ];

        const WINDOW_SIZE: usize = VALID_HEADERS[0].len();

        let _is_seeked = self.seek(SeekFrom::Start(start as u64))?;
        loop {
            let total_ensured = self.ensure(READ_SIZE)?;
            let data = self.read_bytes(READ_SIZE)?;

            if let Some(position) = data
                .windows(WINDOW_SIZE)
                .position(|window| VALID_HEADERS.contains(&window))
            {
                let seek_back = (total_ensured - position) as i64;

                return self.seek(SeekFrom::Current(-seek_back));
            };
        }
    }
}

impl Iterator for ZipFile {
    type Item = std::io::Result<Entry<SectionBody>>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.read_entry())
    }
}
impl NReader for ZipFile {
    fn read_str(&mut self, size: usize) -> std::io::Result<String> {
        let bytes = self.read_bytes(size)?;

        Ok(std::str::from_utf8(bytes)
            .map_err(|_| Into::<std::io::Error>::into(std::io::ErrorKind::InvalidData))?
            .to_string())
    }

    fn read_bytes(&mut self, size: usize) -> std::io::Result<&[u8]> {
        self.__read(size)
    }

    fn read_u8(&mut self) -> std::io::Result<u8> {
        Ok(self.read_bytes(1)?[0])
    }

    fn read_u16(&mut self) -> std::io::Result<u16> {
        Ok(u16::from_le_bytes(self.read_arr(2)?))
    }

    fn read_u32(&mut self) -> std::io::Result<u32> {
        Ok(u32::from_le_bytes(self.read_arr(4)?))
    }

    fn read_u64(&mut self) -> std::io::Result<u64> {
        Ok(u64::from_le_bytes(self.read_arr(8)?))
    }

    fn read_arr<const N: usize>(&mut self, size: usize) -> std::io::Result<[u8; N]> {
        let mut array = [0u8; N];
        let mut _buf = self.__read(size)?;
        _buf.iter().zip(array.iter_mut()).for_each(|(x, y)| *y = *x);

        Ok(array)
    }

    fn __read(&mut self, size: usize) -> std::io::Result<&[u8]> {
        if self.buf_offset == self.buf.len() {
            // the buf is full
            self.buf_offset = 0;
            self.ensure(size)?;
        }

        assert!(self.buf.len() >= (self.buf_offset + size)); // check if the buf len is not ensured

        let result = &self.buf[self.buf_offset..(self.buf_offset + size)];
        self.buf_offset += size;

        Ok(result)
    }
}

impl Debug for Entry<SectionBody> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("entry", &self.entry)
            .field("range", &self.range)
            .finish()
    }
}
impl Read for ZipFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for ZipFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let res = self.file.seek(pos);
        if res.is_ok() {
            self.buf_offset = 0;
        }
        res
    }
}
