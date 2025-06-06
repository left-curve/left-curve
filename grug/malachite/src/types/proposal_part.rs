use {
    crate::{context::Context, ctx, types::RawTx},
    grug::{BorshSerExt, Hash256, SignData, StdError},
    k256::sha2::Sha256,
    malachitebft_core_types::Round,
};

#[grug::derive(Borsh)]
pub struct ProposalInit {
    pub height: ctx!(Height),
    pub round: Round,
    pub proposer: ctx!(Address),
    pub valid_round: Round,
}

impl ProposalInit {
    pub fn new(height: ctx!(Height), round: Round, proposer: ctx!(Address)) -> Self {
        Self {
            height,
            round,
            proposer,
            valid_round: Round::Nil,
        }
    }

    pub fn new_with_valid_round(
        height: ctx!(Height),
        round: Round,
        proposer: ctx!(Address),
        valid_round: Round,
    ) -> Self {
        Self {
            height,
            round,
            proposer,
            valid_round,
        }
    }
}

#[grug::derive(Borsh)]
pub struct ProposalFin {
    pub hash: Hash256,
    pub signature: ctx!(SigningScheme::Signature),
}

impl ProposalFin {
    pub fn new<H, S>(hash: H, signature: S) -> Self
    where
        H: Into<[u8; 32]>,
        S: Into<ctx!(SigningScheme::Signature)>,
    {
        Self {
            hash: Hash256::from_inner(hash.into()),
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
