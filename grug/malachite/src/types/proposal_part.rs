use {crate::context::Context, grug_types::Tx};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalInit;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalFin;

#[derive(Clone, Debug, Eq, PartialEq)]
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
