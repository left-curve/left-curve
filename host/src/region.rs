pub struct Region {
    pub offset:   u32,
    pub capacity: u32,
    pub length:   u32,
}

// note that numbers are stored as little endian
impl Region {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&self.offset.to_le_bytes());
        buf.extend_from_slice(&self.capacity.to_le_bytes());
        buf.extend_from_slice(&self.length.to_le_bytes());
        buf
    }

    pub fn deserialize(buf: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            offset:   u32::from_le_bytes((&buf[0..4]).try_into()?),
            capacity: u32::from_le_bytes((&buf[4..8]).try_into()?),
            length:   u32::from_le_bytes((&buf[8..12]).try_into()?),
        })
    }
}
