use crate::chunks::{
    Chunk, ChunkIterator, Configuration, Entry, KeyAndValue, MapEntry, Spec, Value,
};
use crate::endianness::{LittleEndianU16, LittleEndianU32};
use crate::error::Error;
use crate::stringpool::LoadedStringPool;
use std::collections::HashMap;
use std::mem;
use std::slice;

#[derive(Debug, Clone)]
enum LoadedValue<'bytes> {
    Single(&'bytes Value),
    Complex(&'bytes [KeyAndValue]),
}

#[derive(Debug, Clone)]
struct ConfigAndValue<'bytes>(&'bytes Configuration, LoadedValue<'bytes>);

#[derive(Debug)]
struct LoadedEntry<'bytes> {
    id: u16,
    values: Vec<ConfigAndValue<'bytes>>,
}

#[derive(Debug)]
struct LoadedType<'bytes> {
    id: u8,
    entries: Vec<LoadedEntry<'bytes>>,
}

struct LoadedPackage<'bytes> {
    id: u8,
    name: String,
    #[allow(dead_code)]
    type_strings: LoadedStringPool<'bytes>,
    #[allow(dead_code)]
    name_strings: LoadedStringPool<'bytes>,
    types: Vec<LoadedType<'bytes>>,
}

pub struct LoadedTable<'bytes> {
    _bytes: &'bytes [u8],
    #[allow(dead_code)]
    value_strings: LoadedStringPool<'bytes>,
    packages: Vec<LoadedPackage<'bytes>>,
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
            value_strings,
            packages,
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
        LoadedStringPool::from_chunk(chunk)
    }

    fn parse_package(chunk: Chunk<'bytes>) -> Result<LoadedPackage<'bytes>, Error> {
        let details = chunk.as_package()?;
        let mut type_strings: Option<LoadedStringPool> = None;
        let mut name_strings: Option<LoadedStringPool> = None;
        let mut types: HashMap<u8, Vec<Vec<Option<ConfigAndValue<'bytes>>>>> = HashMap::new();

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
                    LoadedTable::parse_spec(child)?;
                }
                Chunk::Type(_bytes) => {
                    let tt = child.as_type().unwrap().id.value() as u8;
                    let values = LoadedTable::parse_type(child)?;
                    types.entry(tt).or_default();
                    types.entry(tt).and_modify(|e| e.push(values));
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

        let mut loaded_types = Vec::new();
        let mut sorted_ids = types.keys().copied().collect::<Vec<_>>();
        sorted_ids.sort_unstable();
        for id in sorted_ids {
            let all_values = types.get(&id).unwrap();
            let size = all_values.first().unwrap().len();
            let mut loaded_entries: Vec<LoadedEntry<'bytes>> = Vec::with_capacity(size);
            for i in 0..size {
                loaded_entries.push(LoadedEntry {
                    id: i as u16,
                    values: Vec::new(),
                });
            }

            for values in all_values {
                for (i, v) in values.iter().enumerate() {
                    match v {
                        Some(v) => loaded_entries[i].values.push(v.clone()),
                        None => {}
                    }
                }
            }

            loaded_types.push(LoadedType {
                id,
                entries: loaded_entries,
            });
        }

        Ok(LoadedPackage {
            id: details.id.value() as u8,
            name,
            type_strings: type_strings.unwrap(),
            name_strings: name_strings.unwrap(),
            types: loaded_types,
        })
    }

    fn parse_spec(chunk: Chunk<'bytes>) -> Result<&'bytes Spec, Error> {
        let details = chunk.as_spec()?;
        Ok(details)
    }

    fn parse_type(chunk: Chunk<'bytes>) -> Result<Vec<Option<ConfigAndValue<'bytes>>>, Error> {
        let mut values = Vec::new();
        let details = chunk.as_type()?;
        if details.flags.value() & 0x01 != 0 {
            unimplemented!("FLAG_SPARSE not supported yet");
        }
        let config = &details.config;

        let addr: usize = unsafe { mem::transmute(details) };
        let addr = addr + details.header.header_size.value() as usize;
        let payload = unsafe {
            slice::from_raw_parts(
                addr as *const LittleEndianU32,
                details.entry_count.value() as usize,
            )
        };
        for offset in payload.iter() {
            if offset.value() == 0xffff_ffff {
                values.push(None);
            } else {
                let addr: usize = unsafe { mem::transmute(details) };
                let addr = addr + details.entries_offset.value() as usize;
                let addr = addr + offset.value() as usize;
                let entry: &Entry = unsafe { mem::transmute(addr) };

                if entry.flags.value() & 0x01 == 0 {
                    let addr = addr + entry.size.value() as usize;
                    let value: &Value = unsafe { mem::transmute(addr) };
                    values.push(Some(ConfigAndValue(config, LoadedValue::Single(value))));
                } else {
                    let entry: &MapEntry = unsafe { mem::transmute(addr) };
                    let addr = addr + entry.entry.size.value() as usize;
                    let map: &[KeyAndValue] = unsafe {
                        slice::from_raw_parts(
                            addr as *const KeyAndValue,
                            entry.count.value() as usize,
                        )
                    };
                    values.push(Some(ConfigAndValue(config, LoadedValue::Complex(map))));
                }
            }
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::LoadedTable;
    use std::collections::HashSet;

    const RESOURCE_ARSC: &[u8] = include_bytes!("../../tests/data/unpacked/resources.arsc");

    #[test]
    fn parse_valid_table() {
        let table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        assert_eq!(table.packages.len(), 1);

        let actual = (0..table.value_strings.string_count())
            .map(|i| table.value_strings.string_at(i).unwrap())
            .collect::<HashSet<_>>();
        assert!(actual.contains("Foo"));
        assert!(actual.contains("Bar"));
        assert!(actual.contains("Test app"));

        let pkg = &table.packages[0];
        assert_eq!(pkg.id, 0x7f);
        assert_eq!(pkg.name, "test.app".to_owned());
        assert_eq!(pkg.types.len(), 2);

        let mut expected = HashSet::new();
        expected.insert("bool".to_owned());
        expected.insert("string".to_owned());
        let actual = (0..pkg.type_strings.string_count())
            .map(|i| pkg.type_strings.string_at(i).unwrap())
            .collect::<HashSet<_>>();
        assert_eq!(expected, actual);

        let mut expected = HashSet::new();
        expected.insert("app_name".to_owned());
        expected.insert("foo".to_owned());
        let actual = (0..pkg.name_strings.string_count())
            .map(|i| pkg.name_strings.string_at(i).unwrap())
            .collect::<HashSet<_>>();
        assert_eq!(expected, actual);
    }
}
