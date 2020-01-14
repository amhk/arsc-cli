use std::fmt;

pub struct ResourceId {
    pub id: u32,
}

impl ResourceId {
    pub fn from_parts(package_id: u8, type_id: u8, entry_id: u16) -> ResourceId {
        ResourceId {
            id: ((package_id as u32) << 24) | ((type_id as u32) << 16) | entry_id as u32,
        }
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
}
