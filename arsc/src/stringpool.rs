use crate::chunks::{Chunk, StringPool, StringPoolSpan};
use crate::endianness::LittleEndianU32;
use crate::error::Error;
use std::mem;
use std::slice;

#[derive(Debug, Eq, PartialEq)]
pub enum Encoding {
    Utf8,
    Utf16,
}

#[derive(Debug)]
pub struct LoadedStringPool<'bytes> {
    encoding: Encoding,

    string_count: usize,
    string_offsets: &'bytes [LittleEndianU32],
    strings_start: *const u8,

    style_count: usize,
    style_offsets: Option<&'bytes [LittleEndianU32]>,
    styles_start: Option<*const u8>,
}

impl<'bytes> LoadedStringPool<'bytes> {
    pub fn from_chunk(chunk: Chunk<'bytes>) -> Result<LoadedStringPool<'bytes>, Error> {
        let details = chunk.as_stringpool()?;

        if details.header.size.value() < mem::size_of::<StringPool>() as u32 {
            return Err(Error::CorruptData(format!(
                "size {} too small",
                details.header.size.value()
            )));
        }
        if details.header.size.value() < details.header.header_size.value() as u32 {
            return Err(Error::CorruptData(format!(
                "size {} smaller than header size {}",
                details.header.size.value(),
                details.header.header_size.value()
            )));
        }
        if (details.header.header_size.value() as u32 | details.header.size.value()) & 0x03 != 0 {
            return Err(Error::CorruptData("misaligned data".to_owned()));
        }

        let base_addr: usize = unsafe { mem::transmute(details) };
        let addr = base_addr + details.header.header_size.value() as usize;

        let string_offsets: &[LittleEndianU32] = unsafe {
            slice::from_raw_parts(
                addr as *const LittleEndianU32,
                details.string_count.value() as usize,
            )
        };

        let addr = base_addr + details.strings_offset.value() as usize;
        let strings_start = addr as *const u8;

        let style_count = details.style_count.value() as usize;
        let mut style_offsets: Option<&[LittleEndianU32]> = None;
        let styles_start: Option<*const u8> = if style_count != 0 {
            let addr = base_addr
                + details.header.header_size.value() as usize
                + details.string_count.value() as usize * mem::size_of::<LittleEndianU32>();
            style_offsets =
                Some(unsafe { slice::from_raw_parts(addr as *const LittleEndianU32, style_count) });

            Some((base_addr + details.styles_offset.value() as usize) as *const u8)
        } else {
            None
        };

        Ok(LoadedStringPool {
            encoding: if details.flags.value() & (1 << 8) != 0 {
                Encoding::Utf8
            } else {
                Encoding::Utf16
            },

            string_count: details.string_count.value() as usize,
            string_offsets,
            strings_start,

            style_count,
            style_offsets,
            styles_start,
        })
    }

    #[allow(dead_code)]
    pub fn string_count(&self) -> usize {
        self.string_count
    }

    pub fn string_at(&self, i: usize) -> Result<String, Error> {
        if i >= self.string_count {
            return Err(Error::BadIndex);
        }
        match self.encoding {
            Encoding::Utf8 => self.string_at_utf8(i),
            Encoding::Utf16 => self.string_at_utf16(i),
        }
    }

    #[allow(dead_code)]
    pub fn style_count(&self) -> usize {
        self.style_count
    }

    #[allow(dead_code)]
    pub fn style_at(&self, i: usize) -> Result<Vec<LoadedStringPoolSpan>, Error> {
        if i >= self.style_count {
            return Err(Error::BadIndex);
        }

        let style_offsets = self.style_offsets.unwrap();
        let styles_start = self.styles_start.unwrap();
        let mut spans = Vec::new();

        let offset = style_offsets[i].value() as usize;
        unsafe {
            #[allow(clippy::cast_ptr_alignment)]
            let mut span_ptr = styles_start.add(offset) as *const StringPoolSpan;
            while (*span_ptr).name.value() != 0xffff_ffff {
                spans.push(LoadedStringPoolSpan {
                    name: (*span_ptr).name.value(),
                    begin: (*span_ptr).begin.value(),
                    end: (*span_ptr).end.value(),
                });
                span_ptr = span_ptr.add(1);
            }
        }
        Ok(spans)
    }

    fn string_at_utf8(&self, i: usize) -> Result<String, Error> {
        unsafe fn decode_len(ptr: *const u8) -> (usize, usize) {
            let mut len = *ptr as usize;
            if (len & 0x80) != 0 {
                len = (len & 0x7f) << 8 | *ptr.add(1) as usize;
                (2, len)
            } else {
                (1, len)
            }
        }

        unsafe {
            let offset = self.string_offsets[i].value() as usize;
            let string_ptr = self.strings_start.add(offset) as *const u8;

            // length is encoded twice, so fast forward over the first instance
            let (bump, _) = decode_len(string_ptr);
            let string_ptr = string_ptr.add(bump);

            let (bump, len) = decode_len(string_ptr);
            let slice = slice::from_raw_parts(string_ptr.add(bump), len);

            Ok(String::from_utf8_lossy(slice).to_string())
        }
    }

    fn string_at_utf16(&self, i: usize) -> Result<String, Error> {
        unsafe fn decode_len(ptr: *const u16) -> (usize, usize) {
            let mut len = *ptr as usize;
            if (len & 0x8000) != 0 {
                len = (len & 0x7fff) << 16 | *ptr.add(1) as usize;
                (2, len)
            } else {
                (1, len)
            }
        }

        unsafe {
            let offset = self.string_offsets[i].value() as usize;
            #[allow(clippy::cast_ptr_alignment)]
            let string_ptr = self.strings_start.add(offset) as *const u16;

            let (bump, len) = decode_len(string_ptr);
            let slice = slice::from_raw_parts(string_ptr.add(bump), len);

            Ok(String::from_utf16_lossy(slice))
        }
    }
}

#[derive(Debug)]
pub struct LoadedStringPoolSpan {
    pub name: u32,
    pub begin: u32,
    pub end: u32,
}

#[cfg(test)]
mod tests {
    use super::{Encoding, LoadedStringPool};
    use crate::chunks::{Chunk, ChunkIterator};

    const RESOURCE_ARSC: &[u8] = include_bytes!("../../tests/data/unpacked/resources.arsc");

    #[test]
    fn decode_utf8() {
        // find (global) value string pool
        let mut iter = ChunkIterator::new(RESOURCE_ARSC); // entire arsc
        let iter = iter.nth(0).unwrap().iter().unwrap(); // first (and only) table chunk
        let sp = iter
            .filter_map(|chunk| match chunk {
                Chunk::StringPool(_) => Some(LoadedStringPool::from_chunk(chunk).unwrap()),
                _ => None,
            })
            .nth(0)
            .unwrap();

        assert_eq!(sp.encoding, Encoding::Utf8);
        assert_eq!(sp.string_count(), 7);
        assert_eq!(sp.string_at(0).unwrap(), "Foo".to_owned());
        assert_eq!(sp.string_at(1).unwrap(), "Test app".to_owned());
        assert_eq!(sp.string_at(2).unwrap(), "Bar".to_owned());
        // strings at indicies 3-6 are auto-generated "pseudo localized" strings (Foo -> Föö)
        assert!(sp.string_at(3).is_ok());
        assert!(sp.string_at(4).is_ok());
        assert!(sp.string_at(5).is_ok());
        assert!(sp.string_at(6).is_ok());
        assert!(sp.string_at(7).is_err());
    }

    #[test]
    fn decode_utf16() {
        // find (package) type string pool
        let mut iter = ChunkIterator::new(RESOURCE_ARSC); // entire arsc
        let mut iter = iter.nth(0).unwrap().iter().unwrap(); // first (and only) table chunk
        let mut iter = iter.nth(1).unwrap().iter().unwrap(); // first (and only) package chunk
        let sp = iter
            .find_map(|chunk| match chunk {
                Chunk::StringPool(_) => Some(LoadedStringPool::from_chunk(chunk).unwrap()),
                _ => None,
            })
            .unwrap();

        assert_eq!(sp.encoding, Encoding::Utf16);
        assert_eq!(sp.string_count(), 2);
        assert_eq!(sp.string_at(0).unwrap(), "bool".to_string());
        assert_eq!(sp.string_at(1).unwrap(), "string".to_string());
        assert!(sp.string_at(2).is_err());
    }
}
