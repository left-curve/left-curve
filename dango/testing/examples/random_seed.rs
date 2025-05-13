//! Generate a random seed phrase. Derive the corresponding Secp256k1 key pair
//! using Ethereum's derivation path.

use {
    bip32::{Language, Mnemonic, XPrv},
    rand::rngs::OsRng,
};

const HD_PATH: &str = "m/44'/60'/0'/0/0";

fn main() {
    // Generate a random seed phrase
    let mnemonic = Mnemonic::random(OsRng, Language::English);
    let seed = mnemonic.to_seed(""); // empty password
    let path = HD_PATH.parse().unwrap();

    // Seed phrase --> private key
    let sk = XPrv::derive_from_path(seed, &path).unwrap();
    let sk_hex = hex::encode(sk.to_bytes());

    // Private key --> public key
    let pk = sk.public_key();
    let pk_hex = hex::encode(pk.to_bytes());

    println!("Seed phrase:\n{}", mnemonic.phrase());
    println!("\nPrivate key:\n{sk_hex}");
    println!("\nPublic key:\n{pk_hex}");
}
