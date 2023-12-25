use {
    std::mem::size_of,
    wasmi::{
        core::{HostError, Trap},
        AsContext, AsContextMut, Caller,
    },
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

    pub fn deserialize(buf: &[u8]) -> Result<Self, MemoryError> {
        if buf.len() != 12 {
            return Err(MemoryError::ParseRegion(buf.len()));
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

impl<'a, T> TryFrom<&Caller<'a, T>> for Memory {
    type Error = Trap;

    fn try_from(caller: &Caller<'a, T>) -> Result<Self, Trap> {
        caller
            .get_export("memory")
            .ok_or(MemoryError::NoMemoryExport)?
            .into_memory()
            .map(|inner| Self { inner })
            .ok_or(MemoryError::ExportIsNotMemory.into())
    }
}

impl Memory {
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
            return Err(MemoryError::InsufficientRegion {
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
            .map_err(|reason| MemoryError::ReadMemory {
                offset,
                length,
                reason,
            }
            .into())
    }

    fn write(&self, ctx: impl AsContextMut, offset: usize, data: &[u8]) -> Result<(), Trap> {
        self.inner
            .write(ctx, offset, data)
            .map_err(|reason| MemoryError::WriteMemory {
                offset,
                length: data.len(),
                reason,
            }
            .into())
    }
}

// we can't use anyhow::Error, but it doesn't implement wasi::core::HostError
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Can't find an export named `memory`")]
    NoMemoryExport,

    #[error("Failed to cast the memory export to wasmi::Memory type")]
    ExportIsNotMemory,

    #[error("Failed to parse Region: expect 12 bytes, found {0}")]
    ParseRegion(usize),

    #[error("Region too small! capacity: {capacity}, attempting to write: {length}")]
    InsufficientRegion { capacity: u32, length: u32 },

    #[error("Failed to read memory! offset: {offset}, length: {length}, reason: {reason}")]
    ReadMemory {
        offset: usize,
        length: usize,
        reason: wasmi::errors::MemoryError,
    },

    #[error(
        "Failed to write to Wasm memory! offset: {offset}, length: {length}, reason: {reason}"
    )]
    WriteMemory {
        offset: usize,
        length: usize,
        reason: wasmi::errors::MemoryError,
    },
}

// important
impl HostError for MemoryError {}
