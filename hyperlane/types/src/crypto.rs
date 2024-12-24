use {
    crate::{mailbox::Domain, Addr32},
    grug::{Hash256, HashExt, HexByteArray, Inner},
};

pub fn domain_hash(domain: Domain, address: Addr32) -> Hash256 {
    // domain: 4
    // address: 32
    // "HYPERLANE": 9
    // 4 + 32 + 9 = 45
    let mut preimage = [0u8; 45];
    preimage[..4].copy_from_slice(&domain.to_be_bytes());
    preimage[4..36].copy_from_slice(address.inner());
    preimage[36..].copy_from_slice(b"HYPERLANE");
    preimage.keccak256()
}

pub fn multisig_hash(
    domain_hash: Hash256,
    merkle_root: HexByteArray<32>,
    merkle_index: u32,
    message_id: Hash256,
) -> Hash256 {
    // domain_hash: 32
    // merkle_root: 32
    // merkle_index: 4
    // message_id: 32
    // 32 + 32 + 4 + 32 = 100
    let mut preimage = [0u8; 100];
    preimage[..32].copy_from_slice(&domain_hash);
    preimage[32..64].copy_from_slice(&merkle_root);
    preimage[64..68].copy_from_slice(&merkle_index.to_be_bytes());
    preimage[68..].copy_from_slice(&message_id);
    preimage.keccak256()
}

// https://docs.rs/web3/latest/src/web3/signing.rs.html#226-236
pub fn eip191_hash<T>(message: T) -> Hash256
where
    T: AsRef<[u8]>,
{
    let mut preimage = b"\x19Ethereum Signed Message:\n".to_vec();
    preimage.extend(message.as_ref().len().to_string().as_bytes());
    preimage.extend(message.as_ref());
    preimage.keccak256()
}
