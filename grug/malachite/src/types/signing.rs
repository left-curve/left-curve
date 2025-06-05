use {
    crate::{context::Context, ctx},
    malachitebft_core_types::{SignedMessage, SigningProvider},
};

pub type Signature = [u8; 64];
pub type PublicKey = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigningScheme;

impl malachitebft_core_types::SigningScheme for SigningScheme {
    type DecodingError = String;
    type PrivateKey = PrivateKey;
    type PublicKey = PublicKey;
    type Signature = Signature;

    fn decode_signature(bytes: &[u8]) -> Result<Self::Signature, Self::DecodingError> {
        bytes
            .try_into()
            .map_err(|_| "Invalid signature length".to_string())
    }

    fn encode_signature(signature: &Self::Signature) -> Vec<u8> {
        signature.to_vec()
    }
}

#[derive(Clone)]
pub struct PrivateKey([u8; 32]);

impl SigningProvider<Context> for PrivateKey {
    fn sign_vote(&self, vote: ctx!(Vote)) -> SignedMessage<Context, ctx!(Vote)> {
        todo!()
    }

    fn verify_signed_vote(
        &self,
        vote: &ctx!(Vote),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        todo!()
    }

    fn sign_proposal(&self, proposal: ctx!(Proposal)) -> SignedMessage<Context, ctx!(Proposal)> {
        todo!()
    }

    fn verify_signed_proposal(
        &self,
        proposal: &ctx!(Proposal),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        todo!()
    }

    fn sign_proposal_part(
        &self,
        proposal_part: ctx!(ProposalPart),
    ) -> SignedMessage<Context, ctx!(ProposalPart)> {
        todo!()
    }

    fn verify_signed_proposal_part(
        &self,
        proposal_part: &ctx!(ProposalPart),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        todo!()
    }

    fn sign_vote_extension(
        &self,
        extension: ctx!(Extension),
    ) -> SignedMessage<Context, ctx!(Extension)> {
        todo!()
    }

    fn verify_signed_vote_extension(
        &self,
        extension: &ctx!(Extension),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        todo!()
    }
}
