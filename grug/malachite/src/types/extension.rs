use {
    grug::{BorshSerExt, SignData, StdError},
    k256::sha2::Sha256,
};

#[grug::derive(Borsh)]
#[derive(Ord, PartialOrd)]
pub struct Extension;

impl malachitebft_core_types::Extension for Extension {
    fn size_bytes(&self) -> usize {
        0
    }
}

impl SignData for Extension {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> Result<Vec<u8>, Self::Error> {
        self.to_borsh_vec()
    }
}
