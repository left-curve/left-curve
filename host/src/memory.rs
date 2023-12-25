use {
    crate::Region,
    anyhow::{bail, Context},
    std::mem::size_of,
    wasmi::{AsContext, AsContextMut, Caller},
};

/// A wrapper over wasmi::Memory, providing some convenience methods.
#[derive(Debug, Clone, Copy)]
pub struct Memory {
    inner: wasmi::Memory,
}

impl<'a, T> TryFrom<&Caller<'a, T>> for Memory {
    type Error = anyhow::Error;

    fn try_from(caller: &Caller<'a, T>) -> anyhow::Result<Self> {
        let inner = caller
            .get_export("memory")
            .context("Failed to find memory in exports")?
            .into_memory()
            .context("Failed to cast export to memory")?;
        Ok(Self { inner })
    }
}

impl Memory {
    pub fn read_region(
        &self,
        ctx: impl AsContext,
        region_ptr: u32,
    ) -> anyhow::Result<Vec<u8>> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let region = Region::deserialize(&buf)?;

        self.read(ctx, region.offset as usize, region.length as usize)
    }

    pub fn write_region(
        &self,
        mut ctx: impl AsContextMut,
        region_ptr: u32,
        data: &[u8],
    ) -> anyhow::Result<()> {
        let buf = self.read(&ctx, region_ptr as usize, size_of::<Region>())?;
        let mut region = Region::deserialize(&buf)?;
        // don't forget to update the Region length
        region.length = data.len() as u32;

        if region.length > region.capacity {
            bail!(
                "Region too small! Capacity: {}, attempting to write: {}",
                region.capacity,
                region.length,
            );
        }

        self.write(&mut ctx, region.offset as usize, data)?;
        self.write(&mut ctx, region_ptr as usize, &region.serialize())
    }

    fn read(
        &self,
        ctx: impl AsContext,
        offset: usize,
        len: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![0x8; len];
        if let Err(err) = self.inner.read(ctx, offset, &mut buf) {
            bail!(
                "Failed to read memory! offset: {}, length: {}, reason: {}",
                offset,
                len,
                err,
            );
        }
        Ok(buf)
    }

    fn write(
        &self,
        ctx: impl AsContextMut,
        offset: usize,
        data: &[u8],
    ) -> anyhow::Result<()> {
        if let Err(err) = self.inner.write(ctx, offset, data) {
            bail!(
                "Failed to write to Wasm memory! offset: {}, length: {}, reason: {}",
                offset,
                data.len(),
                err,
            );
        }
        Ok(())
    }
}
