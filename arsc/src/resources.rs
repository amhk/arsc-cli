use std::{convert, fmt};

pub struct ResourceId {
    id: u32,
}

impl convert::From<ResourceId> for u32 {
    fn from(resid: ResourceId) -> Self {
        resid.id
    }
}

impl ResourceId {
    pub fn from_parts(package_id: u8, type_id: u8, entry_id: u16) -> ResourceId {
        ResourceId {
            id: ((package_id as u32) << 24) | ((type_id as u32) << 16) | entry_id as u32,
        }
    }

    pub(crate) fn from_u32(id: u32) -> ResourceId {
        ResourceId { id }
    }

    pub fn package_id(&self) -> u8 {
        ((self.id & 0xff00_0000) >> 24) as u8
    }

    pub fn type_id(&self) -> u8 {
        ((self.id & 0x00ff_0000) >> 16) as u8
    }

    pub fn entry_id(&self) -> u16 {
        (self.id & 0x0000_ffff) as u16
    }
}

impl fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceId {{ id: {:#010x} }}", self.id)
    }
}

#[derive(Debug)]
pub enum ResourceValue {
    Null,
    Reference(ResourceId),
    Attribute(ResourceId),
    String(String),
    Float(f32),
    Dimension(f32),
    Fraction(f32),
    IntDec(i32),
    IntHex(i32),
    Boolean(bool),
    ColorArgb8(f32, f32, f32, f32),
    ColorRgb8(f32, f32, f32),
    ColorArgb4(f32, f32, f32, f32),
    ColorRgb4(f32, f32, f32),
    Array(Vec<(ResourceId, ResourceValue)>),
}

pub struct ResourceConfiguration {
    #[allow(dead_code)]
    pub imsi: u32,
    #[allow(dead_code)]
    pub locale: u32,
    #[allow(dead_code)]
    pub screen_type: u32,
    #[allow(dead_code)]
    pub input: u32,
    #[allow(dead_code)]
    pub screen_size: u32,
    #[allow(dead_code)]
    pub version: u32,
    #[allow(dead_code)]
    pub screen_config: u32,
    #[allow(dead_code)]
    pub screen_size_dp: u32,
}

impl fmt::Debug for ResourceConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceConfiguration {{ TODO(#10) }}")
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceId;

    #[test]
    fn from_parts() {
        let resid = ResourceId::from_parts(0x7f, 0x02, 0x0001);
        assert_eq!(resid.id, 0x07f020001);
        assert_eq!(resid.id, ResourceId::from_u32(0x07f020001).id);
    }

    #[test]
    fn parts() {
        let resid = ResourceId::from_u32(0x7f020001);
        assert_eq!(resid.package_id(), 0x7f);
        assert_eq!(resid.type_id(), 0x02);
        assert_eq!(resid.entry_id(), 0x0001);
    }
}
