use crate::endianness::{LittleEndianU16, LittleEndianU32, LittleEndianU8};
use crate::error::Error;
use bitflags::bitflags;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
use std::{fmt, mem};

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u16)]
pub enum ChunkType {
    Null = 0x0000,
    StringPool = 0x0001,
    Table = 0x0002,
    Xml = 0x0003,

    // Xml chunk types
    XmlStartNamespace = 0x0100,
    XmlEndNamespace = 0x0101,
    XmlStartElement = 0x0102,
    XmlEndElement = 0x0103,
    XmlCdata = 0x0104,
    XmlResourceMap = 0x0180,

    // Chunk types following Table
    Package = 0x0200,
    Type = 0x0201,
    Spec = 0x0202,
    Library = 0x0203,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ValueType {
    Null = 0x00,
    Reference = 0x01,
    Attribute = 0x02,
    String = 0x03,
    Float = 0x04,
    Dimension = 0x05,
    Fraction = 0x06,
    DynamicReference = 0x07,
    DynamicAttribute = 0x08,
    IntDec = 0x10,
    IntHex = 0x11,
    IntBoolean = 0x12,
    IntColorArgb8 = 0x1c,
    IntColorRgb8 = 0x1d,
    IntColorArgb4 = 0x1e,
    IntColorRgb4 = 0x1f,
}

#[derive(Debug)]
pub enum Chunk<'arsc> {
    Table(&'arsc [u8]),
    Package(&'arsc [u8]),
    StringPool(&'arsc [u8]),
    Spec(&'arsc [u8]),
    Type(&'arsc [u8]),
    Error(String),
}

impl<'arsc> Chunk<'arsc> {
    pub fn iter(&self) -> Option<ChunkIterator<'arsc>> {
        match self {
            Chunk::Table(bytes) | Chunk::Package(bytes) => {
                #[allow(clippy::transmute_ptr_to_ptr)]
                let header: &Header = unsafe { mem::transmute(&bytes[0]) };
                let inner = &bytes[header.header_size.value() as usize..];
                Some(ChunkIterator::new(inner))
            }
            Chunk::StringPool(_) | Chunk::Spec(_) | Chunk::Type(_) | Chunk::Error(_) => None,
        }
    }

    pub fn as_table(&self) -> Result<&'arsc Table, Error> {
        match *self {
            #[allow(clippy::transmute_ptr_to_ptr)]
            Chunk::Table(bytes) => Ok(unsafe { mem::transmute(&bytes[0]) }),
            _ => Err(Error::UnexpectedChunk),
        }
    }

    pub fn as_package(&self) -> Result<&'arsc Package, Error> {
        match *self {
            #[allow(clippy::transmute_ptr_to_ptr)]
            Chunk::Package(bytes) => Ok(unsafe { mem::transmute(&bytes[0]) }),
            _ => Err(Error::UnexpectedChunk),
        }
    }

    pub fn as_stringpool(&self) -> Result<&'arsc StringPool, Error> {
        match *self {
            #[allow(clippy::transmute_ptr_to_ptr)]
            Chunk::StringPool(bytes) => Ok(unsafe { mem::transmute(&bytes[0]) }),
            _ => Err(Error::UnexpectedChunk),
        }
    }

    pub fn as_spec(&self) -> Result<&'arsc Spec, Error> {
        match *self {
            #[allow(clippy::transmute_ptr_to_ptr)]
            Chunk::Spec(bytes) => Ok(unsafe { mem::transmute(&bytes[0]) }),
            _ => Err(Error::UnexpectedChunk),
        }
    }

    pub fn as_type(&self) -> Result<&'arsc Type, Error> {
        match *self {
            #[allow(clippy::transmute_ptr_to_ptr)]
            Chunk::Type(bytes) => Ok(unsafe { mem::transmute(&bytes[0]) }),
            _ => Err(Error::UnexpectedChunk),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Header {
    pub type_: LittleEndianU16,
    pub header_size: LittleEndianU16,
    pub size: LittleEndianU32,
}

#[repr(C)]
pub struct Configuration {
    size: LittleEndianU32, // size of a Configuration, always 0x40
    pub imsi: LittleEndianU32,
    pub locale: LittleEndianU32,
    pub screen_type: LittleEndianU32,
    pub input: LittleEndianU32,
    pub screen_size: LittleEndianU32,
    pub version: LittleEndianU32,
    pub screen_config: LittleEndianU32,
    pub screen_size_dp: LittleEndianU32,
}

impl fmt::Debug for Configuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut v = Vec::new();
        if self.imsi.value() != 0 {
            v.push(format!("imsi:{:#010x}", self.imsi.value()));
        }
        if self.locale.value() != 0 {
            v.push(format!("locale:{:#010x}", self.locale.value()));
        }
        if self.screen_type.value() != 0 {
            v.push(format!("screen_type:{:#010x}", self.screen_type.value()));
        }
        if self.input.value() != 0 {
            v.push(format!("input:{:#010x}", self.input.value()));
        }
        if self.screen_size.value() != 0 {
            v.push(format!("screen_size:{:#010x}", self.screen_size.value()));
        }
        if self.version.value() != 0 {
            v.push(format!("version:{:#010x}", self.version.value()));
        }
        if self.screen_config.value() != 0 {
            v.push(format!(
                "screen_config:{:#010x}",
                self.screen_config.value()
            ));
        }
        if self.screen_size_dp.value() != 0 {
            v.push(format!(
                "screen_size_dp:{:#010x}",
                self.screen_size_dp.value()
            ));
        }
        if v.is_empty() {
            write!(f, "-")
        } else {
            write!(f, "{}", v.join("-"))
        }
    }
}

bitflags! {
    pub struct ConfigurationFlags: u32 {
        // CONFIG_*
        const MCC = 0x0000_0001;
        const MNC = 0x0000_0002;
        const LOCALE = 0x0000_0004;
        const TOUCHSCREEN = 0x0000_0008;
        const KEYBOARD = 0x0000_0010;
        const KEYBOARD_HIDDEN = 0x0000_0020;
        const NAVIGATION = 0x0000_0040;
        const ORIENTATION = 0x0000_0080;
        const DENSITY = 0x0000_0100;
        const SCREEN_SIZE = 0x0000_0200;
        const SMALLEST_SCREEN_SIZE = 0x0000_2000;
        const VERSION = 0x0000_0400;
        const SCREEN_LAYOUT = 0x0000_0800;
        const UI_MODE = 0x0000_1000;
        const LAYOUTDIR = 0x0000_4000;
        const SCREEN_ROUND = 0x0000_8000;
        const COLOR_MODE = 0x0001_0000;

        // SPEC_PUBLIC
        const PUBLIC = 0x4000_0000;
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Table {
    pub header: Header,
    pub package_count: LittleEndianU32,
}

#[repr(C)]
pub struct Package {
    pub header: Header,
    pub id: LittleEndianU32,
    pub name: [LittleEndianU16; 128],
    pub types_string_buffer_offset: LittleEndianU32,
    _unused_last_public_type: LittleEndianU32,
    pub names_string_buffer_offset: LittleEndianU32,
    _unused_last_public_name: LittleEndianU32,
}

impl fmt::Debug for Package {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Package {{ header: {:?}, id: {:?}, ... }}",
            self.header, self.id
        )
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct StringPool {
    pub header: Header,
    pub string_count: LittleEndianU32,
    pub style_count: LittleEndianU32,
    pub flags: LittleEndianU32,
    pub strings_offset: LittleEndianU32,
    pub styles_offset: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct StringPoolSpan {
    pub name: LittleEndianU32,
    pub begin: LittleEndianU32,
    pub end: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Spec {
    pub header: Header,
    pub id: LittleEndianU8,
    _unused_padding8: LittleEndianU8,
    _unused_padding16: LittleEndianU16,
    pub entry_count: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Type {
    pub header: Header,
    pub id: LittleEndianU8,
    pub flags: LittleEndianU8,
    _unused_padding16: LittleEndianU16,
    pub entry_count: LittleEndianU32,
    pub entries_offset: LittleEndianU32,
    pub config: Configuration,
}

#[derive(Debug)]
#[repr(C)]
pub struct Entry {
    pub size: LittleEndianU16,
    pub flags: LittleEndianU16,
    pub key_index: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct MapEntry {
    pub entry: Entry,
    pub parent_id: LittleEndianU32,
    pub count: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Value {
    pub size: LittleEndianU16, // size of a Value, always 0x08
    _unused_padding8: LittleEndianU8,
    pub type_: LittleEndianU8,
    pub data: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct KeyAndValue {
    pub key: LittleEndianU32,
    pub value: Value,
}

#[derive(Debug)]
pub struct ChunkIterator<'arsc> {
    data: &'arsc [u8],
    offset: usize,
}

impl<'arsc> ChunkIterator<'arsc> {
    pub fn new(data: &'arsc [u8]) -> ChunkIterator<'arsc> {
        ChunkIterator { data, offset: 0 }
    }

    fn invalidate(&mut self) {
        self.offset = self.data.len();
    }
}

impl<'arsc> Iterator for ChunkIterator<'arsc> {
    type Item = Chunk<'arsc>;

    fn next(&mut self) -> Option<Self::Item> {
        // check if iteration is already done
        if self.offset >= self.data.len() {
            return None;
        }

        // read header
        let bytes_left = self.data.len() - self.offset;
        if bytes_left < mem::size_of::<Header>() {
            self.invalidate();
            return Some(Chunk::Error(format!(
                "{:#08x}: {} bytes left cannot contain header",
                self.offset, bytes_left
            )));
        }
        #[allow(clippy::transmute_ptr_to_ptr)]
        let header: &Header = unsafe { mem::transmute(&self.data[self.offset]) };
        let size = header.size.value() as usize;
        let header_size = header.header_size.value() as usize;
        if size < header_size {
            self.invalidate();
            return Some(Chunk::Error(format!(
                "{:#08x}: chunk size {} less than header size {}",
                self.offset, size, header_size
            )));
        }
        if bytes_left < size {
            self.invalidate();
            return Some(Chunk::Error(format!(
                "{:#08x}: {} bytes left cannot contain chunk of {} bytes",
                self.offset, bytes_left, size
            )));
        }
        let type_ = match ChunkType::try_from(header.type_.value()) {
            Ok(t) => t,
            Err(_) => {
                self.invalidate();
                return Some(Chunk::Error(format!(
                    "{:#08x}: unknown chunk type {:#04x}",
                    self.offset,
                    header.type_.value()
                )));
            }
        };

        // advance to next chunk and return
        let bytes = &self.data[self.offset..self.offset + size];
        let chunk = match type_ {
            ChunkType::Table => Chunk::Table(bytes),
            ChunkType::Package => Chunk::Package(bytes),
            ChunkType::StringPool => Chunk::StringPool(bytes),
            ChunkType::Spec => Chunk::Spec(bytes),
            ChunkType::Type => Chunk::Type(bytes),
            _ => todo!("{:?}", type_), // Null, Xml* not handled yet
        };
        self.offset += size;
        Some(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::{Chunk, ChunkIterator, ChunkType, Table};
    use std::convert::TryInto;

    const RESOURCE_ARSC: &[u8] = include_bytes!("../../tests/data/unpacked/resources.arsc");

    #[test]
    fn chunk_type_from_primitive() {
        assert_eq!(0x0200u16.try_into(), Ok(ChunkType::Package));
    }

    #[test]
    fn iter_valid_data() {
        fn iterate(iter: ChunkIterator, depth: usize, out: &mut Vec<String>) {
            for chunk in iter {
                out.push(match chunk {
                    Chunk::Table(_) => format!("{}-Table", depth),
                    Chunk::Package(_) => format!("{}-Package", depth),
                    Chunk::StringPool(_) => format!("{}-StringPool", depth),
                    Chunk::Spec(_) => format!("{}-Spec", depth),
                    Chunk::Type(_) => format!("{}-Type", depth),
                    _ => "ERROR".to_owned(),
                });
                if let Some(child_iter) = chunk.iter() {
                    iterate(child_iter, depth + 1, out);
                }
            }
        }

        let iter = ChunkIterator::new(RESOURCE_ARSC);
        let mut v = Vec::new();
        iterate(iter, 0, &mut v);
        println!("{:#?}", v);
        assert_eq!(
            v,
            [
                "0-Table",
                "1-StringPool",
                "1-Package",
                "2-StringPool",
                "2-StringPool",
                "2-Spec",
                "2-Type",
                "2-Spec",
                "2-Type",
                "2-Type",
                "2-Type",
                "2-Type",
            ]
        );
    }

    #[test]
    fn try_from_chunk_to_table() {
        let mut iter = ChunkIterator::new(RESOURCE_ARSC);
        let chunk = iter.next().unwrap();
        let table: &Table = chunk.as_table().unwrap();
        assert_eq!(table.package_count.value(), 1);
    }
}
