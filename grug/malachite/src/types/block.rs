use {
    crate::{Address, Height, ProposalData, context::Context, ctx, types::RawTx},
    grug::{Defined, Hash256, Inner, PrimaryKey, Timestamp, Undefined},
    k256::{
        elliptic_curve::{consts::U32, generic_array::GenericArray},
        sha2::{Digest, Sha256},
    },
    malachitebft_core_types::{CommitCertificate, Round},
    std::fmt::Display,
};

pub type PreBlock = Block<Undefined<BlockHash>>;

#[grug::derive(Borsh)]
pub struct Block<BH = Defined<BlockHash>> {
    block_hash: BH,
    pub height: Height,
    pub proposer: Address,
    pub round: Round,
    pub timestamp: Timestamp,
    pub txs: Vec<RawTx>,
}

impl PreBlock {
    pub fn new(
        height: ctx!(Height),
        proposer: ctx!(Address),
        round: Round,
        timestamp: Timestamp,
        txs: Vec<RawTx>,
    ) -> Self {
        Self {
            height,
            block_hash: Undefined::new(),
            round,
            proposer,
            txs,
            timestamp,
        }
    }

    pub fn into_block(self, block_hash: BlockHash) -> Block<Defined<BlockHash>> {
        Block {
            block_hash: Defined::new(block_hash),
            height: self.height,
            proposer: self.proposer,
            round: self.round,
            timestamp: self.timestamp,
            txs: self.txs,
        }
    }
}

impl<BH> Block<BH> {
    pub fn as_block_info(&self) -> grug::BlockInfo {
        grug::BlockInfo {
            height: *self.height,
            timestamp: self.timestamp,
            hash: Self::calculate_hash(
                self.height,
                self.proposer,
                self.round,
                self.timestamp,
                &self.txs,
                None,
            ),
        }
    }

    pub fn calculate_block_hash(&self, app_hash: AppHash) -> BlockHash {
        let inner = Self::calculate_hash(
            self.height,
            self.proposer,
            self.round,
            self.timestamp,
            &self.txs,
            Some(app_hash),
        );

        BlockHash(inner)
    }

    fn calculate_hash(
        height: ctx!(Height),
        proposer: ctx!(Address),
        round: Round,
        timestamp: Timestamp,
        txs: &[RawTx],
        app_hash: Option<AppHash>,
    ) -> Hash256 {
        let mut hasher = Sha256::new();

        hasher.update(height.to_be_bytes());
        hasher.update(proposer.as_ref());
        hasher.update(round.as_i64().to_be_bytes());
        hasher.update(timestamp.into_nanos().to_be_bytes());

        for tx in txs {
            hasher.update(tx.as_ref());
        }

        if let Some(app_hash) = app_hash {
            hasher.update(app_hash.0.into_inner());
        }

        Hash256::from_inner(hasher.finalize().into())
    }
}

impl Block {
    pub fn block_hash(&self) -> BlockHash {
        self.block_hash.into_inner()
    }

    pub fn as_proposal_data(&self) -> ProposalData {
        ProposalData {
            block: self.clone(),
            valid_round: Round::Nil,
        }
    }
}

//  --------------------------------- DecidedBlock ---------------------------------

#[grug::derive(Borsh)]
pub struct DecidedBlock {
    pub block: Block,
    pub certificate: CommitCertificate<Context>,
}

//  ---------------------------------AppHash ---------------------------------

#[grug::derive(Borsh)]
pub struct AppHash(Hash256);

impl AppHash {
    pub fn new(hash: Hash256) -> Self {
        Self(hash)
    }
}

impl Display for AppHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

//  ---------------------------------BlockHash ---------------------------------

#[grug::derive(Borsh)]
#[derive(Copy, Ord, PartialOrd)]
pub struct BlockHash(Hash256);

impl From<BlockHash> for GenericArray<u8, U32> {
    fn from(hash: BlockHash) -> Self {
        hash.0.into_inner().into()
    }
}

impl Display for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PrimaryKey for BlockHash {
    type Output = Self;
    type Prefix = ();
    type Suffix = ();

    const KEY_ELEMS: u8 = 1;

    fn raw_keys(&self) -> Vec<grug::RawKey> {
        self.0.raw_keys()
    }

    fn from_slice(bytes: &[u8]) -> grug::StdResult<Self::Output> {
        Hash256::from_slice(bytes).map(Self)
    }
}
