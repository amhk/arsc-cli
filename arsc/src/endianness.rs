#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct LittleEndianU8 {
    value: u8,
}

impl LittleEndianU8 {
    pub fn value(&self) -> u8 {
        u8::from_le(self.value)
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct LittleEndianU16 {
    value: u16,
}

impl LittleEndianU16 {
    pub fn value(&self) -> u16 {
        u16::from_le(self.value)
    }
}

#[derive(Debug, Eq, PartialEq)]
#[repr(C)]
pub struct LittleEndianU32 {
    value: u32,
}

impl LittleEndianU32 {
    pub fn value(&self) -> u32 {
        u32::from_le(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::LittleEndianU8;

    #[test]
    fn little_endian_to_native_endian() {
        let int = LittleEndianU8 {
            value: 32u8.to_le(),
        };
        assert_eq!(int.value(), 32u8);
    }
}
