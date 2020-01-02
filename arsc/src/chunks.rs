use crate::endianness::{LittleEndianU16, LittleEndianU32, LittleEndianU8};
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
    use super::{Chunk, ChunkIterator, ChunkType};
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
}
