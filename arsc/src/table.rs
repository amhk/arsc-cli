use crate::chunks::{
    Chunk, ChunkIterator, Configuration, Entry, KeyAndValue, MapEntry, Spec, Value,
};
use crate::endianness::{LittleEndianU16, LittleEndianU32};
use crate::error::Error;
use crate::resources::ResourceId;
use crate::stringpool::LoadedStringPool;
use std::collections::HashMap;
use std::mem;
use std::slice;

#[derive(Debug, Clone)]
enum LoadedValue<'bytes> {
    Single(&'bytes Entry, &'bytes Value),
    Complex(&'bytes MapEntry, &'bytes [KeyAndValue]),
}

#[derive(Debug, Clone)]
struct ConfigAndValue<'bytes>(&'bytes Configuration, LoadedValue<'bytes>);

#[derive(Debug)]
struct LoadedEntry<'bytes> {
    id: u16,
    name: String,
    values: Vec<ConfigAndValue<'bytes>>,
}

#[derive(Debug)]
struct LoadedType<'bytes> {
    id: u8,
    name: String,
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

    pub fn resid_iter(&self) -> ResourceIdIterator {
        ResourceIdIterator::new(&self)
    }

    pub fn resid_for_name(
        &self,
        package_name: &str,
        type_name: &str,
        entry_name: &str,
    ) -> Option<ResourceId> {
        let p = self.packages.iter().find(|p| p.name == package_name)?;
        let t = p.types.iter().find(|t| t.name == type_name)?;
        let e = t.entries.iter().find(|e| e.name == entry_name)?;
        Some(ResourceId::from_parts(p.id, t.id, e.id))
    }

    pub fn name_for_resid(&self, resid: &ResourceId) -> Option<(String, String, String)> {
        let p = self.packages.iter().find(|p| p.id == resid.package_id())?;
        let t = p.types.iter().find(|t| t.id == resid.type_id())?;
        let e = t.entries.iter().find(|e| e.id == resid.entry_id())?;
        Some((p.name.clone(), t.name.clone(), e.name.clone()))
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

        let type_strings = match type_strings {
            Some(s) => s,
            None => {
                return Err(Error::CorruptData(
                    "missing type string pool in package".to_owned(),
                ))
            }
        };

        let name_strings = match name_strings {
            Some(s) => s,
            None => {
                return Err(Error::CorruptData(
                    "missing name string pool in package".to_owned(),
                ))
            }
        };

        let name = LittleEndianU16::decode_string(&details.name);

        let mut loaded_types = Vec::new();
        let mut sorted_ids = types.keys().copied().collect::<Vec<_>>();
        sorted_ids.sort_unstable();
        for id in sorted_ids {
            let all_values = types.get(&id).unwrap();
            let size = all_values.first().unwrap().len();
            let mut config_and_values: Vec<Vec<ConfigAndValue<'bytes>>> = Vec::new();
            config_and_values.resize_with(size, Vec::new);
            for values in all_values {
                for (i, v) in values.iter().enumerate() {
                    match v {
                        Some(v) => config_and_values[i].push(v.clone()),
                        None => {}
                    }
                }
            }

            let mut entries: Vec<LoadedEntry<'bytes>> = Vec::with_capacity(config_and_values.len());
            while !config_and_values.is_empty() {
                let values = config_and_values.pop().unwrap();
                if values.is_empty() {
                    continue;
                }
                let name = match values.first().unwrap().1 {
                    LoadedValue::Single(entry, _) => name_strings
                        .string_at(entry.key_index.value() as usize)
                        .unwrap(),
                    LoadedValue::Complex(map_entry, _) => name_strings
                        .string_at(map_entry.entry.key_index.value() as usize)
                        .unwrap(),
                };
                entries.push(LoadedEntry {
                    id: config_and_values.len() as u16,
                    name,
                    values,
                });
            }
            entries.sort_unstable_by_key(|entry| entry.id);

            debug_assert!(id > 0);
            loaded_types.push(LoadedType {
                id,
                name: type_strings.string_at((id - 1) as usize)?,
                entries,
            });
        }

        Ok(LoadedPackage {
            id: details.id.value() as u8,
            name,
            type_strings,
            name_strings,
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
                    values.push(Some(ConfigAndValue(
                        config,
                        LoadedValue::Single(entry, value),
                    )));
                } else {
                    let entry: &MapEntry = unsafe { mem::transmute(addr) };
                    let addr = addr + entry.entry.size.value() as usize;
                    let map: &[KeyAndValue] = unsafe {
                        slice::from_raw_parts(
                            addr as *const KeyAndValue,
                            entry.count.value() as usize,
                        )
                    };
                    values.push(Some(ConfigAndValue(
                        config,
                        LoadedValue::Complex(entry, map),
                    )));
                }
            }
        }
        Ok(values)
    }
}

pub struct ResourceIdIterator<'a> {
    iters: Vec<LoadedEntryIterator<'a>>,
    current: Option<LoadedEntryIterator<'a>>,
}

impl<'a> ResourceIdIterator<'a> {
    pub fn new(table: &'a LoadedTable) -> ResourceIdIterator<'a> {
        let mut iters = Vec::new();
        for pkg in &table.packages {
            for type_ in &pkg.types {
                iters.push(LoadedEntryIterator {
                    package_id: pkg.id,
                    type_id: type_.id,
                    entries: type_.entries.iter(),
                });
            }
        }
        iters.reverse(); // items will be popped later: this ensures lowest numbers first
        ResourceIdIterator {
            iters,
            current: None,
        }
    }
}

impl<'a> Iterator for ResourceIdIterator<'a> {
    type Item = ResourceId;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Some(ref mut iter) => match iter.next() {
                Some(resid) => Some(resid),
                None => {
                    self.current = None;
                    self.next()
                }
            },
            None => match self.iters.pop() {
                Some(iter) => {
                    self.current = Some(iter);
                    self.next()
                }
                None => None,
            },
        }
    }
}

struct LoadedEntryIterator<'a> {
    package_id: u8,
    type_id: u8,
    entries: slice::Iter<'a, LoadedEntry<'a>>,
}

impl<'a> Iterator for LoadedEntryIterator<'a> {
    type Item = ResourceId;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| ResourceId::from_parts(self.package_id, self.type_id, entry.id))
    }
}

#[cfg(test)]
mod tests {
    use super::{LoadedPackage, LoadedTable};
    use crate::ResourceId;
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

    #[test]
    fn resid_iter() {
        let table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let expected = vec![0x7f010000, 0x7f020000, 0x7f020001];
        let actual: Vec<u32> = table
            .resid_iter()
            .map(|resid| resid.into())
            .collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn resid_iter_empty_table() {
        let table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let table = LoadedTable {
            packages: Vec::new(),
            ..table
        };
        assert_eq!(0, table.resid_iter().count());
    }

    #[test]
    fn resid_iter_empty_package() {
        let mut table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let pkg = LoadedPackage {
            types: Vec::new(),
            ..table.packages.pop().unwrap()
        };
        let table = LoadedTable {
            packages: vec![pkg],
            ..table
        };
        assert_eq!(0, table.resid_iter().count());
    }

    #[test]
    fn resid_iter_empty_type() {
        let mut table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let mut pkg = table.packages.pop().unwrap();
        pkg.types[0].entries.clear();
        let table = LoadedTable {
            packages: vec![pkg],
            ..table
        };
        let expected = vec![0x7f020000, 0x7f020001];
        let actual: Vec<u32> = table
            .resid_iter()
            .map(|resid| resid.into())
            .collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn resid_iter_two_packages() {
        let mut system_table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let system_pkg = LoadedPackage {
            id: 0x01,
            ..system_table.packages.pop().unwrap()
        };
        let mut table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        let app_pkg = table.packages.pop().unwrap();
        let table = LoadedTable {
            packages: vec![system_pkg, app_pkg],
            ..table
        };
        let expected = vec![
            0x01010000, 0x01020000, 0x01020001, 0x7f010000, 0x7f020000, 0x7f020001,
        ];
        let actual: Vec<u32> = table
            .resid_iter()
            .map(|resid| resid.into())
            .collect::<Vec<_>>();
        assert_eq!(expected, actual);
    }

    #[test]
    fn resid_for_name() {
        let table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        assert_eq!(
            table
                .resid_for_name("test.app", "bool", "foo")
                .map(|resid| resid.into()),
            Some(0x7f010000)
        );
        assert_eq!(
            table
                .resid_for_name("test.app", "string", "app_name")
                .map(|resid| resid.into()),
            Some(0x7f020000)
        );
        assert_eq!(
            table
                .resid_for_name("test.app", "string", "foo")
                .map(|resid| resid.into()),
            Some(0x7f020001)
        );
        assert!(table.resid_for_name("-", "string", "foo").is_none());
        assert!(table.resid_for_name("test.app", "-", "foo").is_none());
        assert!(table.resid_for_name("test.app", "string", "-").is_none());
    }

    #[test]
    fn name_for_resid() {
        let table = LoadedTable::parse(RESOURCE_ARSC).unwrap();
        assert_eq!(
            table.name_for_resid(&ResourceId::from_parts(0x7f, 0x01, 0x0000)),
            Some(("test.app".to_owned(), "bool".to_owned(), "foo".to_owned()))
        );
    }
}
