use std::str::FromStr;

use anyhow::Result;
use grug::{Addressable, BlockInfo, Bound, Empty, ImmutableCtx, Map, MutableCtx, Order, Storage};
use ibc_core_client::types::Height;
use ibc_core_host_types::error::HostError;
use ibc_core_host_types::identifiers::ClientId;
use ibc_core_host_types::path::{
    ClientUpdateHeightPath, ClientUpdateTimePath, ITERATE_CONSENSUS_STATE_PREFIX,
};

/// Travel is an enum to represent the direction of travel in the context of
/// height.
/// Used to get adjacent heights, required by `ibc-rs` light client.
#[derive(Clone, Debug, Copy)]
pub enum HeightTravel {
    /// Next represents the next height.
    Next,
    /// Prev represents the previous height.
    Prev,
}

/// - [`Height`] cannot be used directly as keys in the map since they cannot be easily iterated.
/// - Only a sorted set is needed. So the value type is set to
pub const CONSENSUS_STATE_HEIGHT_MAP: Map<(u64, u64), Empty> =
    Map::new(ITERATE_CONSENSUS_STATE_PREFIX);

enum Ctx<'a> {
    Immutable(ImmutableCtx<'a>),
    Mutable(MutableCtx<'a>),
}

/// Context is a wrapper around the deps and env that provides access
/// to the methods under the ibc-rs Validation and Execution traits.
pub struct TendermintContext<'a> {
    ctx: Ctx<'a>,
    client_id: ClientId,
}

impl<'a> TendermintContext<'a> {
    /// Constructs a new Context object with the given [`ImmutableCtx`].
    /// # Errors
    /// Returns an error if the client id cannot be constructed.
    pub fn new_ref(ctx: ImmutableCtx<'a>) -> Result<Self> {
        let client_id = ClientId::from_str(&ctx.contract.address().to_string())?;

        Ok(Self {
            ctx: Ctx::Immutable(ctx),
            client_id,
        })
    }

    /// Constructs a new Context object with the given [`MutableCtx`].
    /// # Errors
    /// Returns an error if the client id cannot be constructed.
    pub fn new_mut(ctx_mut: MutableCtx<'a>) -> Result<Self> {
        let client_id = ClientId::from_str(&ctx_mut.contract.address().to_string())?;

        Ok(Self {
            ctx: Ctx::Mutable(ctx_mut),
            client_id,
        })
    }

    /// Returns the client id of the context.
    #[must_use]
    pub fn client_id(&self) -> ClientId {
        self.client_id.clone()
    }

    /// Prefixes the given key with the migration prefix.
    pub fn prefixed_key(&self, key: impl AsRef<[u8]>) -> Vec<u8> {
        // No prefixing is required for the key at the moment.
        let mut prefixed_key = Vec::new();
        prefixed_key.extend_from_slice(key.as_ref());

        prefixed_key
    }

    /// Retrieves the value of the given key.
    /// # Errors
    /// Returns an error if the key is not found.
    pub fn retrieve(&self, key: impl AsRef<[u8]>) -> Result<Vec<u8>, HostError> {
        let prefixed_key = self.prefixed_key(key);

        let value = self
            .storage_ref()
            .read(prefixed_key.as_ref())
            .ok_or_else(|| HostError::failed_to_retrieve("key not found upon retrieval"))?;

        Ok(value)
    }

    /// Inserts the given key-value pair.
    pub fn insert(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
        self.storage_mut().write(key.as_ref(), value.as_ref());
    }

    /// Removes the value of the given key.
    pub fn remove(&mut self, key: impl AsRef<[u8]>) {
        self.storage_mut().remove(key.as_ref());
    }

    /// Returns the storage of the context.
    /// # Errors
    /// Returns an error if the storage is not available.
    pub fn get_heights(&self) -> Result<Vec<Height>, HostError> {
        CONSENSUS_STATE_HEIGHT_MAP
            .keys(self.storage_ref(), None, None, Order::Ascending)
            .map(|deserialized_result| {
                let (rev_number, rev_height) =
                    deserialized_result.map_err(HostError::failed_to_retrieve)?;
                Height::new(rev_number, rev_height).map_err(HostError::invalid_state)
            })
            .collect()
    }

    /// Searches for either the earliest next or latest previous height based on
    /// the given height and travel direction.
    /// # Errors
    /// Returns an error if the storage is not available.
    pub fn get_adjacent_height(
        &self,
        height: &Height,
        travel: HeightTravel,
    ) -> Result<Option<Height>, HostError> {
        let iterator = match travel {
            HeightTravel::Prev => CONSENSUS_STATE_HEIGHT_MAP.range(
                self.storage_ref(),
                None,
                Some(Bound::Exclusive((
                    height.revision_number(),
                    height.revision_height(),
                ))),
                Order::Descending,
            ),
            HeightTravel::Next => CONSENSUS_STATE_HEIGHT_MAP.range(
                self.storage_ref(),
                Some(Bound::Exclusive((
                    height.revision_number(),
                    height.revision_height(),
                ))),
                None,
                Order::Ascending,
            ),
        };

        iterator
            .map(|deserialized_result| {
                let ((rev_number, rev_height), _) =
                    deserialized_result.map_err(HostError::failed_to_retrieve)?;
                Height::new(rev_number, rev_height).map_err(HostError::invalid_state)
            })
            .next()
            .transpose()
    }

    /// Returns the key for the client update time.
    #[must_use]
    pub fn client_update_time_key(&self, height: &Height) -> Vec<u8> {
        let client_update_time_path = ClientUpdateTimePath::new(
            self.client_id(),
            height.revision_number(),
            height.revision_height(),
        );

        client_update_time_path.leaf().into_bytes()
    }

    /// Returns the key for the client update height.
    #[must_use]
    pub fn client_update_height_key(&self, height: &Height) -> Vec<u8> {
        let client_update_height_path = ClientUpdateHeightPath::new(
            self.client_id(),
            height.revision_number(),
            height.revision_height(),
        );

        client_update_height_path.leaf().into_bytes()
    }

    /// Returns the storage reference of the context.
    #[must_use]
    pub fn storage_ref(&self) -> &dyn Storage {
        match self.ctx {
            Ctx::Mutable(ref ctx_mut) => ctx_mut.storage,
            Ctx::Immutable(ref ctx) => ctx.storage,
        }
    }

    /// Returns the mutable storage of the context.
    /// # Panics
    /// Panics if the mutable context is not available.
    pub fn storage_mut(&mut self) -> &mut dyn Storage {
        match self.ctx {
            Ctx::Mutable(ref mut ctx_mut) => ctx_mut.storage,
            Ctx::Immutable(_) => panic!("Mutable context should be available"),
        }
    }

    /// Returns the block info of the context.
    #[must_use]
    pub const fn block(&self) -> BlockInfo {
        match self.ctx {
            Ctx::Mutable(ref ctx_mut) => ctx_mut.block,
            Ctx::Immutable(ref ctx) => ctx.block,
        }
    }
}
