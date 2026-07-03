use dango_primitives::FullBlockCompat;

/// The block payload that flows through the [`BlockSource`] and is consumed by
/// projections: the full `Block` together with its `BlockOutcome`, in
/// whichever wire schema the block was written.
///
/// This is the node's own [`dango_primitives::FullBlockCompat`], reused
/// directly rather than duplicated. Two properties fall out of that for free:
///
/// - **Wire compatibility by construction.** The `full_block` subscription and
///   the `/block/full/*` REST routes serialize the same compat shape
///   (`{ block, outcome }`, untagged), so what we deserialize is *exactly*
///   what the node serializes — including the seven historical blocks whose
///   `Message::Configure` predates the 0.26.0 taxman removal and only exists
///   in the legacy layout (see [`dango_primitives::legacy`]).
/// - **On-disk format.** The compat enum derives borsh; the detached
///   `RemoteBlockSource` persists it to its raw store with the enum's
///   one-byte version tag, and falls back to the bare (untagged) layout for
///   values written before the tag existed.
///
/// `BlockData` is kept as the indexer's domain name for the payload (the alias
/// the `BlockSource` trait and the projections speak in).
///
/// [`BlockSource`]: dango_archive_block_source::BlockSource
pub type BlockData = FullBlockCompat;

/// `height()` ergonomics for [`BlockData`].
///
/// The compat enum's inherent accessors live in `dango-primitives`; the height
/// shorthand the source and projections lean on lives here as an extension
/// trait — in scope wherever it is called.
pub trait BlockDataExt {
    /// The block height, taken from `block.info`.
    fn height(&self) -> u64;
}

impl BlockDataExt for BlockData {
    fn height(&self) -> u64 {
        self.info().height
    }
}
