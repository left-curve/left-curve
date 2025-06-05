use {crate::context::Context, grug::Tx};

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
