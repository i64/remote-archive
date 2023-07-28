use binrw::{helpers::count, BinRead};
use exact_reader::ExactReader;
use std::{
    fmt::Debug,
    io::{Read, Seek, SeekFrom},
};

use crate::file_types::EntryIter;

use super::{Entry, FileType};

const PAGE_SIZE: usize = 2_usize.pow(16);
pub struct ZipFile<R> {
    parser: ZipParser<R>,
    initalized: bool,
}

impl<R: Read + Seek> ZipFile<R> {
    pub fn new(reader: ExactReader<R>) -> Self {
        let file_size = reader.size() as usize;
        let parser = ZipParser { file_size, reader };
        Self {
            parser,
            initalized: false,
        }
    }

    pub fn find_eocd(&mut self) -> std::io::Result<()> {
        todo!()
    }
}

impl<R: Read + Seek> FileType for ZipFile<R> {
    fn entry_iter(&mut self) -> std::io::Result<super::EntryIter<Self>> {
        self.parser
            .reader
            .seek(std::io::SeekFrom::End(-(PAGE_SIZE as i64)))?;

        let _ = self
            .parser
            .try_find_section(SectionType::EndOfCentralDir)
            .unwrap();

        let eocd = self.parser.end_of_central_dir()?;

        if eocd.disk_num == u16::MAX
            || eocd.cd_start_disk == u16::MAX
            || eocd.num_records_on_disk == u16::MAX
            || eocd.num_records == u16::MAX
            || eocd.cd_size == u32::MAX
            || eocd.cd_start_offset == u32::MAX
            || eocd.comment_length == u16::MAX
        {
            self.parser
                .reader
                .seek(std::io::SeekFrom::Current(-(PAGE_SIZE as i64)))?;

            let _ = self
                .parser
                .try_find_section(SectionType::EndOfCentralDir64Locator)
                .unwrap();

            let eocd_locator = self.parser.end_of_central_dir64_locator()?;
            self.parser
                .reader
                .seek(std::io::SeekFrom::Start(eocd_locator.zip64_eocd_offset))?;

            let eocd64 = self.parser.end_of_central_dir64()?;

            self.parser
                .reader
                .seek(std::io::SeekFrom::Start(eocd64.cd_start_offset))?;

            self.parser.reader.reserve(eocd64.cd_size as usize);

            Ok(EntryIter {
                source: self,
                count: eocd64.num_records as usize,
            })
        } else {
            self.parser
                .reader
                .seek(std::io::SeekFrom::Start(eocd.cd_start_offset as u64))?;

            self.parser.reader.reserve(eocd.cd_size as usize);

            Ok(EntryIter {
                source: self,
                count: eocd.num_records as usize,
            })
        }
    }
}

impl<R: Read + Seek> Iterator for EntryIter<'_, ZipFile<R>> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            self.count -= 1;
            let cd = self.source.parser.central_dir_entry().ok()?;
            Some(Self::Item {
                filename: cd.filename,
            })
        } else {
            None
        }
    }
}
struct ZipParser<R> {
    reader: ExactReader<R>,
    file_size: usize,
}

impl<R: Read + Seek> ZipParser<R> {
    fn try_find_section(&mut self, section_type: SectionType) -> Option<u64> {
        let (_, section_magic) = section_type.section_info();

        let k;

        let data = {
            let _ = self.reader.reserve(PAGE_SIZE);
            k = self.reader.stream_position().unwrap();
            let mut data: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
            self.reader.read(&mut data).unwrap();
            data
        };

        if let Some(position) = data
            .windows(section_magic.len())
            .position(|window| window == section_magic)
        {
            let seek_back = (PAGE_SIZE - position) as i64;
            let new_position = self.reader.seek(SeekFrom::Current(-seek_back)).unwrap();
            return Some(new_position);
        };

        None
    }
    fn central_dir_entry(&mut self) -> std::io::Result<CentralDirEntry> {
        self.reader.reserve(SectionType::MIN_CENTRAL_DIR_SIZE);
        let res = CentralDirEntry::read(&mut self.reader).unwrap();
        self.reader.seek(std::io::SeekFrom::Current(
            (res.extra_field_length + res.comment_length) as i64,
        ));
        Ok(res)
    }


    fn end_of_central_dir64_locator(&mut self) -> std::io::Result<EndOfCentralDir64Locator> {
        self.reader
            .reserve(SectionType::MIN_END_OF_CENTRAL_DIR64_LOCATOR_SIZE);
        Ok(EndOfCentralDir64Locator::read(&mut self.reader).unwrap())
    }

    fn end_of_central_dir64(&mut self) -> std::io::Result<EndOfCentralDir64> {
        self.reader
            .reserve(SectionType::MIN_END_OF_CENTRAL_DIR64_SIZE);
        Ok(EndOfCentralDir64::read(&mut self.reader).unwrap())
    }

    fn end_of_central_dir(&mut self) -> std::io::Result<EndOfCentralDir> {
        self.reader
            .reserve(SectionType::MIN_END_OF_CENTRAL_DIR_SIZE);

        Ok(EndOfCentralDir::read(&mut self.reader).unwrap())
    }
}

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

#[derive(Debug)]
#[repr(u32)]
enum SectionType {
    // DataDescriptor = 0x08074b50,
    // LocalFile = 0x04034b50,
    CentralDirEntry = 0x02014b50,
    EndOfCentralDir = 0x06054b50,
    EndOfCentralDir64 = 0x06064b50,
    EndOfCentralDir64Locator = 0x07064b50,
}

impl SectionType {
    const MIN_CENTRAL_DIR_SIZE: usize = 2 * 6 + 4 * 3 + 2 * 5 + 4 * 2;
    const MIN_END_OF_CENTRAL_DIR_SIZE: usize = std::mem::size_of::<EndOfCentralDir>();
    const MIN_END_OF_CENTRAL_DIR64_SIZE: usize = std::mem::size_of::<EndOfCentralDir64>();
    const MIN_END_OF_CENTRAL_DIR64_LOCATOR_SIZE: usize =
        std::mem::size_of::<EndOfCentralDir64Locator>();
    const fn section_info(self) -> (usize, [u8; 4]) {
        let section_size: usize = match self {
            Self::CentralDirEntry => Self::MIN_CENTRAL_DIR_SIZE,
            Self::EndOfCentralDir => Self::MIN_END_OF_CENTRAL_DIR_SIZE,
            Self::EndOfCentralDir64 => Self::MIN_END_OF_CENTRAL_DIR64_SIZE,
            Self::EndOfCentralDir64Locator => Self::MIN_END_OF_CENTRAL_DIR64_LOCATOR_SIZE,
        };

        let section_magic: [u8; 4] = (self as u32).to_le_bytes();

        (section_size, section_magic)
    }
}

back_to_enum! {
    #[derive(Debug, BinRead)]
    #[br(repr = u16)]
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

#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(little, magic = 0x02014b50u32)]
pub struct CentralDirEntry {
    version_made_by: u16,
    version_needed: u16,
    gpb_flags: u16,
    compression_method: u16,
    last_mod_time: u16,
    last_mod_date: u16,
    crc32: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    file_name_length: u16,
    extra_field_length: u16,
    comment_length: u16,
    file_start_disk: u16,
    internal_file_attributes: u16,
    external_file_attributes: u32,
    local_file_header_offset: u32,
    #[br(count = file_name_length, map = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())]
    filename: String,
}
#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(little, magic = 0x06054b50u32)]
pub struct EndOfCentralDir {
    disk_num: u16,
    cd_start_disk: u16,
    num_records_on_disk: u16,
    num_records: u16,
    cd_size: u32,
    cd_start_offset: u32,
    comment_length: u16,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(little, magic = 0x06064b50u32)]
pub struct EndOfCentralDir64 {
    record_size: u64,
    version_made_by: u16,
    version_needed: u16,
    disk_num: u32,
    cd_start_disk: u32,
    num_records_on_disk: u64,
    num_records: u64,
    cd_size: u64,
    cd_start_offset: u64,
}

#[allow(dead_code)]
#[derive(Debug, BinRead)]
#[br(little, magic = 0x07064b50u32)]
pub struct EndOfCentralDir64Locator {
    // disk_of_end_of_central_dir: u16,
    eocd_start_disk: u32,
    zip64_eocd_offset: u64,
    num_of_disks: u32,
}