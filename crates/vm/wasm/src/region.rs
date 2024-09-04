use {crate::VmError, std::mem, wasmer::ValueType};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub offset: u32,
    pub capacity: u32,
    pub length: u32,
}

unsafe impl ValueType for Region {
    fn zero_padding_bytes(&self, _bytes: &mut [mem::MaybeUninit<u8>]) {
        // there is no padding in Region so not necessary.
    }
}

#[derive(Clone, Copy)]
pub struct UncheckedRegion {
    pub offset: u32,
    pub capacity: u32,
    pub length: u32,
}

unsafe impl ValueType for UncheckedRegion {
    fn zero_padding_bytes(&self, _bytes: &mut [mem::MaybeUninit<u8>]) {
        // there is no padding in Region so not necessary.
    }
}

impl TryFrom<UncheckedRegion> for Region {
    type Error = VmError;

    fn try_from(unchecked: UncheckedRegion) -> Result<Self, Self::Error> {
        if unchecked.offset == 0 {
            return Err(VmError::RegionZeroOffset {});
        }
        if unchecked.length > unchecked.capacity {
            return Err(VmError::RegionLengthExceedsCapacity {
                length: unchecked.length,
                capacity: unchecked.capacity,
            });
        }
        if unchecked.capacity > (u32::MAX - unchecked.offset) {
            return Err(VmError::RegionOutOfRange {
                offset: unchecked.offset,
                capacity: unchecked.capacity,
            });
        }
        Ok(Region {
            offset: unchecked.offset,
            capacity: unchecked.capacity,
            length: unchecked.length,
        })
    }
}
