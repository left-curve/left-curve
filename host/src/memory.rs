use {
    crate::Error,
    anyhow::Context,
    std::mem::size_of,
    wasmi::{core::Trap, AsContext, AsContextMut, Caller, Instance, Store},
};

/// Parallel to sdk::Region
#[derive(Debug)]
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

    pub fn deserialize(buf: &[u8]) -> Result<Self, Error> {
        if buf.len() != 12 {
            return Err(Error::ParseRegion(buf.len()));
        }

        Ok(Self {
            offset:   u32::from_le_bytes((&buf[0..4]).try_into().unwrap()),
            capacity: u32::from_le_bytes((&buf[4..8]).try_into().unwrap()),
            length:   u32::from_le_bytes((&buf[8..12]).try_into().unwrap()),
        })
    }
}

/// A wrapper over wasmi::Memory, providing some convenience methods.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    inner: wasmi::Memory,
}

impl<T> TryFrom<(&Instance, &Store<T>)> for Memory {
    type Error = anyhow::Error;

    fn try_from((instance, store): (&Instance, &Store<T>)) -> anyhow::Result<Self> {
        instance
            .get_memory(store, "memory")
            .map(Self::new)
            .context("Failed to get memory from instance")
    }
}

impl<'a, T> TryFrom<&Caller<'a, T>> for Memory {
    type Error = Trap;

    fn try_from(caller: &Caller<'a, T>) -> Result<Self, Trap> {
        caller
            .get_export("memory")
            .ok_or(Error::ExportNotFound)?
            .into_memory()
            .map(Self::new)
            .ok_or(Error::ExportIsNotMemory.into())
    }
}

impl Memory {
    pub fn new(inner: wasmi::Memory) -> Self {
        Self { inner }
    }

    pub fn read_region(&self, ctx: impl AsContext, region_ptr: u32) -> Result<Vec<u8>, Trap> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read(ctx, region.offset as usize, region.length as usize)
    }

    pub fn write_region(
        &self,
        mut ctx: impl AsContextMut,
        region_ptr: u32,
        data: &[u8],
    ) -> Result<(), Trap> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let mut region = Region::deserialize(&buf)?;
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            return Err(Error::InsufficientRegion {
                capacity: region.capacity,
                length:   region.length,
            }
            .into());
        }

        self.write(&mut ctx, region.offset as usize, data)?;
        self.write(&mut ctx, region_ptr as usize, &region.serialize())
    }

    fn read(&self, ctx: impl AsContext, offset: usize, length: usize) -> Result<Vec<u8>, Trap> {
        let mut buf = vec![0x8; length];
        self.inner
            .read(ctx, offset, &mut buf)
            .map(|_| buf)
            .map_err(|reason| Error::ReadMemory {
                offset,
                length,
                reason,
            }
            .into())
    }

    fn write(&self, ctx: impl AsContextMut, offset: usize, data: &[u8]) -> Result<(), Trap> {
        self.inner
            .write(ctx, offset, data)
            .map_err(|reason| Error::WriteMemory {
                offset,
                length: data.len(),
                reason,
            }
            .into())
    }
}
