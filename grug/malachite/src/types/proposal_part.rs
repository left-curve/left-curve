use {
    crate::context::Context,
    grug::{BorshSerExt, SignData, StdError, Tx},
    k256::sha2::Sha256,
};

#[grug::derive(Borsh)]
pub struct ProposalInit;

#[grug::derive(Borsh)]
pub struct ProposalFin;

#[grug::derive(Borsh)]
pub enum ProposalPart {
    Init(ProposalInit),
    Data(Vec<Tx>),
    Fin(ProposalFin),
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
