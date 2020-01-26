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

#[cfg(test)]
mod tests {
    use super::ResourceId;

    #[test]
    fn from_parts() {
        let resid = ResourceId::from_parts(0x7f, 0x02, 0x0001);
        assert_eq!(resid.id, 0x07f020001);
    }

    #[test]
    fn parts() {
        let resid = ResourceId::from_parts(0x7f, 0x02, 0x0001);
        assert_eq!(resid.package_id(), 0x7f);
        assert_eq!(resid.type_id(), 0x02);
        assert_eq!(resid.entry_id(), 0x0001);
    }
}
