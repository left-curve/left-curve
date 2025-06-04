pub type Signature = [u8; 64];
pub type PublicKey = [u8; 32];
pub type PrivateKey = [u8; 32];

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
