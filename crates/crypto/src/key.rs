use std::fmt;

/// Elliptic curves.
///
/// In CWD we provide support for two curves: secp256k1 and r1. If you need to
/// work with other curves, please let the devs know.
#[derive(Clone)]
pub enum Curve {
    Secp256k1,
    Secp256r1,
}

impl fmt::Display for Curve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Curve::Secp256k1 => f.write_str("secp256k1"),
            Curve::Secp256r1 => f.write_str("secp256r1"),
        }
    }
}

pub enum SigningKey {
    Secp256k1(k256::ecdsa::SigningKey),
    Secp256r1(p256::ecdsa::SigningKey),
}

impl SigningKey {
    // TODO
}
