use {
    std::ptr::NonNull,
    wasmer::{
        MemoryType, Pages, TableType, Tunables,
        vm::{
            MemoryError, MemoryStyle, TableStyle, VMMemory, VMMemoryDefinition, VMTable,
            VMTableDefinition,
        },
    },
};

const ERR_MIN_EXCEEDS_ALLOWED: &str = "minimum exceeds the allowed memory limit";

const ERR_MAX_EXCEEDS_ALLOWED: &str = "maximum exceeds the allowed memory limit";

const ERR_MAX_UNSET: &str = "maximum unset";

/// A custom tunables that allows to set a memory limit.
///
/// After adjusting the memory limits, it delegates all other logic to the base
/// tunables.
pub struct LimitingTunables<T> {
    /// The base implementation we delegate all the logic to.
    base: T,
    /// The maximum a linear memory is allowed to be (in Wasm pages, 65 KiB each).
    /// Since Wasmer ensures there is only none or one memory, this is practically
    /// an upper limit for the guest memory.
    limit: Pages,
}

impl<T> LimitingTunables<T> {
    pub fn new(base: T, limit: u32) -> Self {
        Self {
            base,
            limit: Pages(limit),
        }
    }

    /// Takes in input memory type as requested by the guest and sets a maximum
    /// if missing. The resulting memory type is final if valid. However, this
    /// can produce invalid types, such that `validate_memory` must be called
    /// before creating the memory.
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
            return Err(MemoryError::Generic(ERR_MIN_EXCEEDS_ALLOWED.to_string()));
        }

        let Some(max) = ty.maximum else {
            return Err(MemoryError::Generic(ERR_MAX_UNSET.to_string()));
        };

        if max > self.limit {
            return Err(MemoryError::Generic(ERR_MAX_EXCEEDS_ALLOWED.to_string()));
        }

        Ok(())
    }
}

impl<T> Tunables for LimitingTunables<T>
where
    T: Tunables,
{
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
    /// The requested memory type is validated, adjusted to the limited and then
    /// passed to base.
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

        unsafe {
            self.base
                .create_vm_memory(&adjusted, style, vm_definition_location)
        }
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
        unsafe { self.base.create_vm_table(ty, style, vm_definition_location) }
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        test_case::test_case,
        wasmer::{Target, sys::BaseTunables},
    };

    const MOCK_LIMIT: u32 = 12;

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
        let base = BaseTunables::for_target(&Target::default());
        let tunables = LimitingTunables::new(base, MOCK_LIMIT);
        assert_eq!(
            tunables.adjust_memory(&requested),
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
        Err(MemoryError::Generic(ERR_MAX_EXCEEDS_ALLOWED.to_string()));
        "maximum greater than limit"
    )]
    #[test_case(
        MemoryType::new(3, None, true),
        Err(MemoryError::Generic(ERR_MAX_UNSET.to_string()));
        "maximum not set"
    )]
    #[test_case(
        MemoryType::new(5, Some(3), true),
        Ok(());
        "minimum greater than maximum"
    )]
    #[test_case(
        MemoryType::new(20, Some(20), true),
        Err(MemoryError::Generic(ERR_MIN_EXCEEDS_ALLOWED.to_string()));
        "minimum greater than limit"
    )]
    fn validate_memory(request: MemoryType, result: Result<(), MemoryError>) {
        let base = BaseTunables::for_target(&Target::default());
        let tunables = LimitingTunables::new(base, MOCK_LIMIT);
        assert_eq!(tunables.validate_memory(&request), result);
    }
}
