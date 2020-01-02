use crate::chunks::{Chunk, ChunkIterator};
use crate::error::Error;

// dummy struct (for now)
struct StringPool {}
struct Package {}

pub struct Table<'bytes> {
    bytes: &'bytes [u8],
    value_strings: StringPool,
    packages: Vec<Package>,
}

impl<'bytes> Table<'bytes> {
    pub fn parse(bytes: &[u8]) -> Result<Table, Error> {
        let mut iter = ChunkIterator::new(bytes);
        let chunk = match iter.next() {
            Some(Chunk::Table(b)) => Chunk::Table(b),
            Some(x) => return Err(Error::CorruptData(format!("not a table chunk: {:?}", x))),
            None => return Err(Error::CorruptData("no data to traverse".to_owned())),
        };
        if iter.next().is_some() {
            return Err(Error::CorruptData("trailing data after table".to_owned()));
        }
        let (value_strings, packages) = Table::parse_table(&chunk)?;
        Ok(Table {
            bytes,
            value_strings,
            packages,
        })
    }

    fn parse_table(chunk: &'bytes Chunk) -> Result<(StringPool, Vec<Package>), Error> {
        let details = chunk.as_table()?;
        let mut packages = Vec::<Package>::new();
        let mut value_strings: Option<StringPool> = None;

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
                    value_strings = Some(Table::parse_stringpool(&child)?);
                }
                Chunk::Package(_) => {
                    packages.push(Table::parse_package(&child)?);
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

    fn parse_stringpool(chunk: &'bytes Chunk) -> Result<StringPool, Error> {
        // FIXME: implement this
        let _details = chunk.as_stringpool()?;
        Ok(StringPool {})
    }

    fn parse_package(chunk: &'bytes Chunk) -> Result<Package, Error> {
        // FIXME: implement this
        let _details = chunk.as_package()?;
        let iter = chunk
            .iter()
            .ok_or_else(|| Error::CorruptData("cannot iterate over package".to_owned()))?;
        for child in iter {
            match child {
                Chunk::Spec(_bytes) => {
                    let _spec_details = child.as_spec()?;
                }
                Chunk::Type(_bytes) => {
                    let _type_details = child.as_type()?;
                }
                _ => {}
            }
        }
        Ok(Package {})
    }
}

#[cfg(test)]
mod tests {
    use super::Table;

    const RESOURCE_ARSC: &[u8] = include_bytes!("../../tests/data/unpacked/resources.arsc");

    #[test]
    fn parse_valid_table() {
        let table = Table::parse(RESOURCE_ARSC).unwrap();
    }
}
