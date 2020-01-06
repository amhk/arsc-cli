use crate::chunks::{
    Chunk, ChunkIterator, ConfigurationFlags, Entry, KeyAndValue, MapEntry, Spec, Value,
};
use crate::endianness::{LittleEndianU16, LittleEndianU32};
use crate::error::Error;
use std::mem;
use std::slice;

// dummy struct (for now)
struct LoadedStringPool {}

struct LoadedPackage<'bytes> {
    _specs: Vec<&'bytes Spec>,
}

pub struct LoadedTable<'bytes> {
    _bytes: &'bytes [u8],
    _value_strings: LoadedStringPool,
    _packages: Vec<LoadedPackage<'bytes>>,
}

impl<'bytes> LoadedTable<'bytes> {
    pub fn parse(bytes: &'bytes [u8]) -> Result<LoadedTable<'bytes>, Error> {
        let mut iter = ChunkIterator::new(bytes);
        let chunk = match iter.next() {
            Some(Chunk::Table(b)) => Chunk::Table(b),
            Some(x) => return Err(Error::CorruptData(format!("not a table chunk: {:?}", x))),
            None => return Err(Error::CorruptData("no data to traverse".to_owned())),
        };
        if iter.next().is_some() {
            return Err(Error::CorruptData("trailing data after table".to_owned()));
        }
        let (value_strings, packages) = LoadedTable::parse_table(chunk)?;
        Ok(LoadedTable {
            _bytes: bytes,
            _value_strings: value_strings,
            _packages: packages,
        })
    }

    fn parse_table(
        chunk: Chunk<'bytes>,
    ) -> Result<(LoadedStringPool, Vec<LoadedPackage<'bytes>>), Error> {
        let details = chunk.as_table()?;
        let mut packages = Vec::<LoadedPackage<'bytes>>::new();
        let mut value_strings: Option<LoadedStringPool> = None;

        let iter = chunk
            .iter()
            .ok_or_else(|| Error::CorruptData("cannot iterate over table".to_owned()))?;
        for child in iter {
            match child {
                Chunk::StringPool(_) => {
                    if value_strings.is_some() {
                        return Err(Error::CorruptData(
                            "muiltiple string pools in table".to_owned(),
                        ));
                    }
                    value_strings = Some(LoadedTable::parse_stringpool(child)?);
                }
                Chunk::Package(_) => {
                    packages.push(LoadedTable::parse_package(child)?);
                }
                _ => return Err(Error::UnexpectedChunk),
            }
        }

        if value_strings.is_none() {
            return Err(Error::CorruptData(
                "missing string pool in table".to_owned(),
            ));
        }

        if packages.len() != details.package_count.value() as usize {
            return Err(Error::CorruptData(format!(
                "expected {} packages, found {}",
                details.package_count.value(),
                packages.len()
            )));
        }

        Ok((value_strings.unwrap(), packages))
    }

    fn parse_stringpool(chunk: Chunk<'bytes>) -> Result<LoadedStringPool, Error> {
        // FIXME: implement this
        let _details = chunk.as_stringpool()?;
        Ok(LoadedStringPool {})
    }

    fn parse_package(chunk: Chunk<'bytes>) -> Result<LoadedPackage<'bytes>, Error> {
        let details = chunk.as_package()?;
        let mut type_strings: Option<LoadedStringPool> = None;
        let mut name_strings: Option<LoadedStringPool> = None;
        let mut specs: Vec<&'bytes Spec> = Vec::new();

        let iter = chunk
            .iter()
            .ok_or_else(|| Error::CorruptData("cannot iterate over package".to_owned()))?;
        for child in iter {
            match child {
                Chunk::StringPool(_bytes) => {
                    let child_details = child.as_stringpool()?;

                    let base_addr: usize = unsafe { mem::transmute(details) };
                    let child_addr: usize = unsafe { mem::transmute(child_details) };
                    let offset = child_addr - base_addr;

                    if offset == details.types_string_buffer_offset.value() as usize {
                        if type_strings.is_some() {
                            return Err(Error::CorruptData(
                                "multiple type string pools".to_owned(),
                            ));
                        }
                        type_strings = Some(LoadedTable::parse_stringpool(child)?);
                    } else if offset == details.names_string_buffer_offset.value() as usize {
                        if name_strings.is_some() {
                            return Err(Error::CorruptData(
                                "multiple name string pools".to_owned(),
                            ));
                        }
                        name_strings = Some(LoadedTable::parse_stringpool(child)?);
                    } else {
                        return Err(Error::CorruptData(
                            "unexpected string pool in package".to_owned(),
                        ));
                    }
                }
                Chunk::Spec(_bytes) => {
                    specs.push(LoadedTable::parse_spec(child)?);
                }
                Chunk::Type(_bytes) => {
                    LoadedTable::parse_type(child)?;
                }
                _ => return Err(Error::UnexpectedChunk),
            }
        }

        if type_strings.is_none() {
            return Err(Error::CorruptData(
                "missing type string pool in package".to_owned(),
            ));
        }

        if name_strings.is_none() {
            return Err(Error::CorruptData(
                "missing name string pool in package".to_owned(),
            ));
        }

        let name = LittleEndianU16::decode_string(&details.name);
        println!("package id={:#04x} name={:?}", details.id.value(), name);

        Ok(LoadedPackage { _specs: specs })
    }

    fn parse_spec(chunk: Chunk<'bytes>) -> Result<&'bytes Spec, Error> {
        let details = chunk.as_spec()?;
        println!(
            "spec id={:#04x} entry_count={}",
            details.id.value(),
            details.entry_count.value()
        );

        let addr: usize = unsafe { mem::transmute(details) };
        let addr = addr + details.header.header_size.value() as usize;
        let payload = unsafe {
            slice::from_raw_parts(
                addr as *const LittleEndianU32,
                details.entry_count.value() as usize,
            )
        };

        for le in payload.iter() {
            match ConfigurationFlags::from_bits(le.value()) {
                None => {
                    return Err(Error::CorruptData(format!(
                        "bad CONFIG_* bitmask {:#010x}",
                        le.value()
                    )))
                }
                Some(flags) => println!("    {:?}", flags),
            }
        }

        Ok(details)
    }

    fn parse_type(chunk: Chunk<'bytes>) -> Result<(), Error> {
        let details = chunk.as_type()?;
        if details.flags.value() & 0x01 != 0 {
            unimplemented!("FLAG_SPARSE not supported yet");
        }

        println!(
            "    type id={:#04x} flags={:#04x} entry_count={} config={:?}",
            details.id.value(),
            details.flags.value(),
            details.entry_count.value(),
            details.config
        );

        let addr: usize = unsafe { mem::transmute(details) };
        let addr = addr + details.header.header_size.value() as usize;
        let payload = unsafe {
            slice::from_raw_parts(
                addr as *const LittleEndianU32,
                details.entry_count.value() as usize,
            )
        };
        for (i, offset) in payload.iter().enumerate() {
            println!("        entry={:#06x} offset={:#010x}", i, offset.value());
            if offset.value() == 0xffff_ffff {
                println!("            no entry");
            } else {
                let addr: usize = unsafe { mem::transmute(details) };
                let addr = addr + details.entries_offset.value() as usize;
                let addr = addr + offset.value() as usize;
                let entry: &Entry = unsafe { mem::transmute(addr) };
                println!("            key_index={:#010x}", entry.key_index.value());

                if entry.flags.value() & 0x01 == 0 {
                    let addr = addr + entry.size.value() as usize;
                    let value: &Value = unsafe { mem::transmute(addr) };
                    println!(
                        "            type={:#04x} data={:#010x}",
                        value.type_.value(),
                        value.data.value()
                    );
                } else {
                    let entry: &MapEntry = unsafe { mem::transmute(addr) };
                    println!(
                        "            map.parent_id={:#010x} map.count={}",
                        entry.parent_id.value(),
                        entry.count.value()
                    );
                    let addr = addr + entry.entry.size.value() as usize;
                    let map: &[KeyAndValue] = unsafe {
                        slice::from_raw_parts(
                            addr as *const KeyAndValue,
                            entry.count.value() as usize,
                        )
                    };
                    for (i, pair) in map.iter().enumerate() {
                        println!(
                            "            map[{}]: key={:#010x} type={:#04x} data={:#010x}",
                            i,
                            pair.key.value(),
                            pair.value.type_.value(),
                            pair.value.data.value()
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::LoadedTable;

    const RESOURCE_ARSC: &[u8] = include_bytes!("../../tests/data/unpacked/resources.arsc");

    #[test]
    fn parse_valid_table() {
        let _table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        assert_eq!(_table._packages.len(), 1);

        let pkg = &_table._packages[0];
        assert_eq!(pkg._specs.len(), 2);
    }
}
