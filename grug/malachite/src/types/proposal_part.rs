use {
    crate::{
        context::Context,
        ctx,
        types::{BlockHash, PreBlock, RawTx},
    },
    grug::{BorshSerExt, SignData, StdError, Timestamp},
    k256::sha2::Sha256,
    malachitebft_core_types::Round,
};

#[grug::derive(Borsh)]
pub struct ProposalInit {
    pub height: ctx!(Height),
    pub round: Round,
    pub proposer: ctx!(Address),
    pub timestamp: Timestamp,
    pub valid_round: Round,
}

impl ProposalInit {
    pub fn new(
        height: ctx!(Height),
        proposer: ctx!(Address),
        round: Round,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            height,
            round,
            proposer,
            timestamp,
            valid_round: Round::Nil,
        }
    }

    pub fn new_with_valid_round(
        height: ctx!(Height),
        proposer: ctx!(Address),
        round: Round,
        timestamp: Timestamp,
        valid_round: Round,
    ) -> Self {
        Self {
            height,
            round,
            proposer,
            timestamp,
            valid_round,
        }
    }
}

#[grug::derive(Borsh)]
pub struct ProposalFin {
    pub hash: BlockHash,
    pub signature: ctx!(SigningScheme::Signature),
}

impl ProposalFin {
    pub fn new<S>(hash: BlockHash, signature: S) -> Self
    where
        S: Into<ctx!(SigningScheme::Signature)>,
    {
        Self {
            hash,
            signature: signature.into(),
        }
    }
}

#[grug::derive(Borsh)]
pub enum ProposalPart {
    Init(ProposalInit),
    Data(Vec<RawTx>),
    Fin(ProposalFin),
}

impl ProposalPart {
    pub fn as_init(&self) -> Option<&ProposalInit> {
        match self {
            Self::Init(init) => Some(init),
            _ => None,
        }
    }

    pub fn as_fin(&self) -> Option<&ProposalFin> {
        match self {
            Self::Fin(fin) => Some(fin),
            _ => None,
        }
    }
}

impl malachitebft_core_types::ProposalPart<Context> for ProposalPart {
    fn is_first(&self) -> bool {
        matches!(self, ProposalPart::Init(_))
    }

    fn is_last(&self) -> bool {
        matches!(self, ProposalPart::Fin(_))
    }
}

impl SignData for ProposalPart {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> Result<Vec<u8>, Self::Error> {
        self.to_borsh_vec()
    }
}

#[grug::derive(Borsh)]
pub struct ProposalParts {
    pub init: ProposalInit,
    pub data: Vec<RawTx>,
    pub fin: ProposalFin,
}

impl ProposalParts {
    pub fn into_pre_block(self) -> PreBlock {
        PreBlock::new(
            self.init.height,
            self.init.proposer,
            self.init.round,
            self.init.timestamp,
            self.data,
        )
    }
}

impl IntoIterator for ProposalParts {
    type IntoIter = <Vec<ProposalPart> as IntoIterator>::IntoIter;
    type Item = ProposalPart;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            ProposalPart::Init(self.init),
            ProposalPart::Data(self.data),
            ProposalPart::Fin(self.fin),
        ]
        .into_iter()
    }
}
