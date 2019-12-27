use crate::endianness::{LittleEndianU16, LittleEndianU32, LittleEndianU8};
use num_enum::TryFromPrimitive;
use std::fmt;

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

#[derive(Debug)]
#[repr(C)]
pub struct Header {
    pub type_: LittleEndianU16,
    pub header_size: LittleEndianU16,
    pub size: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Configuration {
    size: LittleEndianU32,
    imsi: LittleEndianU32,
    locale: LittleEndianU32,
    screen_type: LittleEndianU32,
    input: LittleEndianU32,
    screen_size: LittleEndianU32,
    version: LittleEndianU32,
    screen_config: LittleEndianU32,
    screen_size_dp: LittleEndianU32,
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
pub struct Spec {
    pub header: Header,
    pub id: LittleEndianU8,
    _unused_padding8: LittleEndianU8,
    _unused_padding16: LittleEndianU16,
    pub entry_count: LittleEndianU32,
    pub flags: LittleEndianU32,
}

#[derive(Debug)]
#[repr(C)]
pub struct Type {
    pub header: Header,
    pub id: LittleEndianU8,
    pub flags: LittleEndianU8,
    _unused_padding16: LittleEndianU16,
    pub entry_count: LittleEndianU32,
    pub entires_offset: LittleEndianU32,
    pub config: Configuration,
}

#[cfg(test)]
mod tests {
    use super::ChunkType;
    use std::convert::TryInto;

    #[test]
    fn chunk_type_from_primitive() {
        assert_eq!(0x0200u16.try_into(), Ok(ChunkType::Package));
    }
}
