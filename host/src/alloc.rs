use {
    crate::Error,
    wasmi::{core::Trap, AsContextMut, Caller, Instance, Store, TypedFunc},
};

pub struct Allocator {
    alloc_fn:   TypedFunc<u32, u32>,
    dealloc_fn: TypedFunc<u32, ()>,
}

impl<T> TryFrom<(&Instance, &Store<T>)> for Allocator {
    type Error = anyhow::Error;

    fn try_from((instance, store): (&Instance, &Store<T>)) -> anyhow::Result<Self> {
        let alloc_fn = instance.get_typed_func(store, "allocate")?;
        let dealloc_fn = instance.get_typed_func(store, "deallocate")?;
        Ok(Self { alloc_fn, dealloc_fn })
    }
}

impl<'a, T> TryFrom<&Caller<'a, T>> for Allocator {
    type Error = Trap;

    fn try_from(caller: &Caller<'a, T>) -> Result<Self, Trap> {
        let alloc_fn = caller
            .get_export("allocate")
            .ok_or(Error::ExportNotFound)?
            .into_func()
            .ok_or(Error::ExportIsNotFunc)?
            .typed(&caller)
            .map_err(Error::from)?;
        let dealloc_fn = caller
            .get_export("deallocate")
            .ok_or(Error::ExportNotFound)?
            .into_func()
            .ok_or(Error::ExportIsNotFunc)?
            .typed(&caller)
            .map_err(Error::from)?;
        Ok(Self { alloc_fn, dealloc_fn })
    }
}

impl Allocator {
    pub fn allocate(&self, ctx: impl AsContextMut, capacity: usize) -> Result<u32, Trap> {
        self.alloc_fn.call(ctx, capacity as u32)
    }

    pub fn deallocate(&self, ctx: impl AsContextMut, region_ptr: u32) -> Result<(), Trap> {
        self.dealloc_fn.call(ctx, region_ptr)
    }
}
