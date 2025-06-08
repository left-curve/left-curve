use {
    crate::{
        context::Context,
        ctx,
        types::{Address, Height, ProposalFin, ProposalInit, ProposalParts, RawTx},
    },
    grug::{Defined, Hash256, Inner, PrimaryKey, Timestamp, Undefined},
    k256::{
        elliptic_curve::{consts::U32, generic_array::GenericArray},
        sha2::{Digest, Sha256},
    },
    malachitebft_core_types::{CommitCertificate, Round},
    std::fmt::Display,
};

pub type PreBlock = Block<Undefined<AppHash>>;

#[grug::derive(Borsh)]
pub struct Block<AH = Defined<AppHash>> {
    pub app_hash: AH,
    pub height: Height,
    pub proposer: Address,
    pub round: Round,
    pub txs: Vec<RawTx>,
    pub timestamp: Timestamp,
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
            app_hash: Undefined::new(),
            round,
            proposer,
            txs,
            timestamp,
        }
    }

    pub fn with_app_hash(self, app_hash: AppHash) -> Block<Defined<AppHash>> {
        Block {
            app_hash: Defined::new(app_hash),
            height: self.height,
            round: self.round,
            proposer: self.proposer,
            txs: self.txs,
            timestamp: self.timestamp,
        }
    }
}

impl<T> Block<T> {
    pub fn pre_hash(&self) -> PreHash {
        let mut hasher = Sha256::new();

        hasher.update(self.height.to_be_bytes());
        hasher.update(self.proposer.as_ref());
        hasher.update(self.round.as_i64().to_be_bytes());
        hasher.update(self.timestamp.into_nanos().to_be_bytes());

        for tx in &self.txs {
            hasher.update(tx.as_ref());
        }

        PreHash(Hash256::from_inner(hasher.finalize().into()))
    }

    pub fn as_block_info(&self) -> grug::BlockInfo {
        grug::BlockInfo {
            height: *self.height,
            timestamp: self.timestamp,
            hash: self.pre_hash().0,
        }
    }
}

impl Block {
    pub fn hash(&self) -> BlockHash {
        let mut hasher = Sha256::new();

        hasher.update(self.height.to_be_bytes());
        hasher.update(self.proposer.as_ref());
        hasher.update(self.round.as_i64().to_be_bytes());
        hasher.update(self.timestamp.into_nanos().to_be_bytes());

        for tx in &self.txs {
            hasher.update(tx.as_ref());
        }

        hasher.update(self.app_hash.inner().0);

        BlockHash(Hash256::from_inner(hasher.finalize().into()))
    }

    pub fn as_parts(&self, private_key: &ctx!(SigningScheme::PrivateKey)) -> ProposalParts {
        let hash = self.hash();
        let signature = private_key.sign_digest(hash);

        ProposalParts {
            init: ProposalInit::new(self.height, self.proposer, self.round, self.timestamp),
            data: self.txs.clone(),
            fin: ProposalFin::new(hash, signature),
        }
    }

    pub fn override_app_hash(&mut self, app_hash: AppHash) {
        self.app_hash = Defined::new(app_hash);
    }
}

//  --------------------------------- DecidedBlock ---------------------------------

#[grug::derive(Borsh)]
pub struct DecidedBlock {
    pub block: Block,
    pub certificate: CommitCertificate<Context>,
}

//  --------------------------------- PreHash ----------------------------------

#[grug::derive(Borsh)]
pub struct PreHash(Hash256);

impl Display for PreHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
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
