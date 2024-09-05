use {
    std::ptr::NonNull,
    wasmer::{
        vm::{
            MemoryError, MemoryStyle, TableStyle, VMMemory, VMMemoryDefinition, VMTable,
            VMTableDefinition,
        },
        MemoryType, Pages, TableType, Tunables, WASM_PAGE_SIZE,
    },
};

/// Wasm memory limit in `MiB`.
pub const WASM_MEMORY_LIMIT: usize = 32;
/// Wasm memory limit in `Pages`.
pub const WASM_PAGES_LIMIT: Pages =
    Pages((WASM_MEMORY_LIMIT * 1024 * 1024 / WASM_PAGE_SIZE) as u32);

/// A custom tunables that allows to set a memory limit.
///
/// After adjusting the memory limits, it delegates all other logic
/// to the base tunables.
pub struct LimitingTunables<T: Tunables> {
    /// The maxium a linear memory is allowed to be (in Wasm pages, 65 KiB each).
    /// Since Wasmer ensures there is only none or one memory, this is practically
    /// an upper limit for the guest memory.
    limit: Pages,
    /// The base implementation we delegate all the logic to
    base: T,
}

impl<T: Tunables> LimitingTunables<T> {
    pub fn new(base: T, limit: Pages) -> Self {
        Self { limit, base }
    }

    /// Takes in input memory type as requested by the guest and sets
    /// a maximum if missing. The resulting memory type is final if
    /// valid. However, this can produce invalid types, such that
    /// validate_memory must be called before creating the memory.
    fn adjust_memory(&self, requested: &MemoryType) -> MemoryType {
        let mut adjusted = *requested;
        if requested.maximum.is_none() {
            adjusted.maximum = Some(self.limit);
        }
        adjusted
    }

    /// Ensures the a given memory type does not exceed the memory limit.
    /// Call this after adjusting the memory.
    fn validate_memory(&self, ty: &MemoryType) -> Result<(), MemoryError> {
        if ty.minimum > self.limit {
            return Err(MemoryError::Generic(
                "Minimum exceeds the allowed memory limit".to_string(),
            ));
        }

        if let Some(max) = ty.maximum {
            if max > self.limit {
                return Err(MemoryError::Generic(
                    "Maximum exceeds the allowed memory limit".to_string(),
                ));
            }
        } else {
            return Err(MemoryError::Generic("Maximum unset".to_string()));
        }

        Ok(())
    }
}

impl<T: Tunables> Tunables for LimitingTunables<T> {
    /// Construct a `MemoryStyle` for the provided `MemoryType`
    ///
    /// Delegated to base.
    fn memory_style(&self, memory: &MemoryType) -> MemoryStyle {
        let adjusted = self.adjust_memory(memory);
        self.base.memory_style(&adjusted)
    }

    /// Construct a `TableStyle` for the provided `TableType`
    ///
    /// Delegated to base.
    fn table_style(&self, table: &TableType) -> TableStyle {
        self.base.table_style(table)
    }

    /// Create a memory owned by the host given a [`MemoryType`] and a [`MemoryStyle`].
    ///
    /// The requested memory type is validated, adjusted to the limited and then passed to base.
    fn create_host_memory(
        &self,
        ty: &MemoryType,
        style: &MemoryStyle,
    ) -> Result<VMMemory, MemoryError> {
        let adjusted = self.adjust_memory(ty);
        self.validate_memory(&adjusted)?;
        self.base.create_host_memory(&adjusted, style)
    }

    /// Create a memory owned by the VM given a [`MemoryType`] and a [`MemoryStyle`].
    ///
    /// Delegated to base.
    unsafe fn create_vm_memory(
        &self,
        ty: &MemoryType,
        style: &MemoryStyle,
        vm_definition_location: NonNull<VMMemoryDefinition>,
    ) -> Result<VMMemory, MemoryError> {
        let adjusted = self.adjust_memory(ty);
        self.validate_memory(&adjusted)?;
        self.base
            .create_vm_memory(&adjusted, style, vm_definition_location)
    }

    /// Create a table owned by the host given a [`TableType`] and a [`TableStyle`].
    ///
    /// Delegated to base.
    fn create_host_table(&self, ty: &TableType, style: &TableStyle) -> Result<VMTable, String> {
        self.base.create_host_table(ty, style)
    }

    /// Create a table owned by the VM given a [`TableType`] and a [`TableStyle`].
    ///
    /// Delegated to base.
    unsafe fn create_vm_table(
        &self,
        ty: &TableType,
        style: &TableStyle,
        vm_definition_location: NonNull<VMTableDefinition>,
    ) -> Result<VMTable, String> {
        self.base.create_vm_table(ty, style, vm_definition_location)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        test_case::test_case,
        wasmer::{sys::BaseTunables, Target},
    };

    #[test_case(
        MemoryType::new(3, None, true),
        Some(MemoryType::new(3, Some(12), true));
        "no maximum"
    )]
    #[test_case(
        MemoryType::new(3, Some(7), true),
        None;
        "maximum smaller than limit"
    )]
    #[test_case(
        MemoryType::new(3, Some(12), true),
        None;
        "maximum equal to limit"
    )]
    #[test_case(
        MemoryType::new(3, Some(20), true),
        None;
        "maximum greater than limit"
    )]
    #[test_case(
        MemoryType::new(5, Some(3), true),
        None;
        "minimum greater than maximum"
    )]
    #[test_case(
        MemoryType::new(20, Some(20), true),
        None;
        "minimum greater than limit"
    )]
    fn adjust_memory(requested: MemoryType, compare: Option<MemoryType>) {
        let limit = Pages(12);
        let limiting = LimitingTunables::new(BaseTunables::for_target(&Target::default()), limit);
        assert_eq!(
            limiting.adjust_memory(&requested),
            compare.unwrap_or(requested)
        );
    }

    #[test_case(
        MemoryType::new(3, Some(7), true),
        Ok(());
        "maximum smaller than limit"
    )]
    #[test_case(
        MemoryType::new(3, Some(12), true),
        Ok(());
        "maximum equal to limit"
    )]
    #[test_case(
        MemoryType::new(3, Some(20), true),
        Err(MemoryError::Generic("Maximum exceeds the allowed memory limit".to_string()));
        "maximum greater than limit"
    )]
    #[test_case(
        MemoryType::new(3, None, true),
        Err(MemoryError::Generic("Maximum unset".to_string()));
        "maximum not set"
    )]
    #[test_case(
        MemoryType::new(5, Some(3), true),
        Ok(());
        "minimum greater than maximum"
    )]
    #[test_case(
        MemoryType::new(20, Some(20), true),
        Err(MemoryError::Generic("Minimum exceeds the allowed memory limit".to_string()));
        "minimum greater than limit"
    )]
    fn validate_memory(request: MemoryType, result: Result<(), MemoryError>) {
        let limit = Pages(12);
        let limiting = LimitingTunables::new(BaseTunables::for_target(&Target::default()), limit);
        assert_eq!(result, limiting.validate_memory(&request));
    }
}
