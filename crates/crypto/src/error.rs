use grug_types::CryptoError;

pub type CryptoResult<T> = core::result::Result<T, CryptoError>;

pub(crate) trait SignatureResultExt {
    type Inner;
    fn crypto_verify_failed(self, verify_fn: &'static str) -> CryptoResult<Self::Inner>;
    fn crypto_recovery_failed(self, verify_fn: &'static str) -> CryptoResult<Self::Inner>;
    fn crypto_invalid_pk_format(self, verify_fn: &'static str) -> CryptoResult<Self::Inner>;
    fn crypto_invalid_sig_format(self, verify_fn: &'static str) -> CryptoResult<Self::Inner>;
}

impl<T> SignatureResultExt for Result<T, signature::Error> {
    type Inner = T;

    fn crypto_verify_failed(self, verify_fn: &'static str) -> CryptoResult<T> {
        self.map_err(|_| CryptoError::VerifyFailed(verify_fn.to_string()))
    }

    fn crypto_recovery_failed(self, verify_fn: &'static str) -> CryptoResult<Self::Inner> {
        self.map_err(|_| CryptoError::RecoveryFailed(verify_fn.to_string()))
    }

    fn crypto_invalid_pk_format(self, verify_fn: &'static str) -> CryptoResult<T> {
        self.map_err(|_| CryptoError::InvalidPk(verify_fn.to_string()))
    }

    fn crypto_invalid_sig_format(self, verify_fn: &'static str) -> CryptoResult<T> {
        self.map_err(|_| CryptoError::InvalidSig(verify_fn.to_string()))
    }
}
