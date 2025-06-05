use {
    crate::{context::Context, ctx},
    grug::{SignData, StdError},
    grug_crypto::Identity256,
    k256::{
        ecdsa::signature::{DigestSigner, DigestVerifier},
        sha2::Sha256,
    },
    malachitebft_core_types::{SignedMessage, SigningProvider},
    std::fmt::Debug,
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
pub struct PrivateKey(k256::ecdsa::SigningKey);

impl PrivateKey {
    pub fn sign<T>(&self, data: T) -> SignedMessage<Context, T>
    where
        T: SignData<Hasher = Sha256>,
        T::Error: Debug,
    {
        let sign_data = data.to_sign_data().unwrap();
        let sign_data = Identity256::from_inner(sign_data);
        let signature: k256::ecdsa::Signature = self.0.sign_digest(sign_data);
        SignedMessage::new(data, signature.to_bytes().into())
    }

    pub fn verify<T>(
        &self,
        data: &T,
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool
    where
        T: SignData<Hasher = Sha256, Error = StdError>,
        T::Error: Debug,
    {
        (|| {
            let sign_data = data.to_sign_data().unwrap();
            let sign_data = Identity256::from_inner(sign_data);
            let signature = k256::ecdsa::Signature::from_bytes(signature.into())?;
            let public_key = k256::ecdsa::VerifyingKey::from_sec1_bytes(public_key)?;
            public_key.verify_digest(sign_data, &signature)
        })()
        .is_ok()
    }
}

impl SigningProvider<Context> for PrivateKey {
    fn sign_vote(&self, vote: ctx!(Vote)) -> SignedMessage<Context, ctx!(Vote)> {
        self.sign(vote)
    }

    fn verify_signed_vote(
        &self,
        vote: &ctx!(Vote),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        self.verify(vote, signature, public_key)
    }

    fn sign_proposal(&self, proposal: ctx!(Proposal)) -> SignedMessage<Context, ctx!(Proposal)> {
        self.sign(proposal)
    }

    fn verify_signed_proposal(
        &self,
        proposal: &ctx!(Proposal),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        self.verify(proposal, signature, public_key)
    }

    fn sign_proposal_part(
        &self,
        proposal_part: ctx!(ProposalPart),
    ) -> SignedMessage<Context, ctx!(ProposalPart)> {
        self.sign(proposal_part)
    }

    fn verify_signed_proposal_part(
        &self,
        proposal_part: &ctx!(ProposalPart),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        self.verify(proposal_part, signature, public_key)
    }

    fn sign_vote_extension(
        &self,
        extension: ctx!(Extension),
    ) -> SignedMessage<Context, ctx!(Extension)> {
        self.sign(extension)
    }

    fn verify_signed_vote_extension(
        &self,
        extension: &ctx!(Extension),
        signature: &ctx!(SigningScheme::Signature),
        public_key: &ctx!(SigningScheme::PublicKey),
    ) -> bool {
        self.verify(extension, signature, public_key)
    }
}
