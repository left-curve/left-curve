use {
    crate::{Binary, Hash, MapKey, RawKey},
    serde::{Deserialize, Serialize},
    sha2::{Digest, Sha256},
    std::{
        fmt,
        ops::{Deref, DerefMut},
        str::FromStr,
    },
};

// comparing addresses in cosmwasm vs in CWD
// - in cosmwasm: 20 bytes, bech32 encoding, Addr is a wrapper of String
// - in CWD: 32 bytes, hex encoding (lowercase, no checksum, with 0x prefix),
//   Addr is a wrapper of [u8; 32]
//
// unlike in cosmwasm, where you need to use deps.api.addr_validate to verify an
// address, in CWD, Addrs are verified at deserialization time. therefore it's
// ok to use Addrs in APIs (i.e. messages and query responses).
// haven't benchmarked the performance impact, by deserializing hex should be
// much cheaper than bech32?
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Addr(Hash);

impl Addr {
    pub fn new(hash: Hash) -> Self {
        Self(hash)
    }

    /// Compute a contract address as:
    /// blake3(deployer_addr | code_hash | salt),
    /// where | means byte concatenation.
    pub fn compute(deployer: &Addr, code_hash: &Hash, salt: &Binary) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(deployer);
        hasher.update(code_hash);
        hasher.update(salt);
        Self(Hash(hasher.finalize().into()))
    }

    /// Generate a mock address from use in testing.
    pub const fn mock(index: u8) -> Self {
        let mut bytes = [0u8; Hash::LENGTH];
        bytes[Hash::LENGTH - 1] = index;
        Self(Hash(bytes))
    }
}

impl AsRef<[u8]> for Addr {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for Addr {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl Deref for Addr {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Addr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TryFrom<&[u8]> for Addr {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Hash::try_from(bytes).map(Self)
    }
}

impl FromStr for Addr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Hash::from_str(s).map(Self)
    }
}

impl MapKey for &Addr {
    type Prefix = ();
    type Suffix = ();
    type Output = Addr;

    fn raw_keys(&self) -> Vec<RawKey> {
        vec![RawKey::Ref(self.0.as_ref())]
    }

    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self::Output> {
        bytes.try_into()
    }
}

impl From<Addr> for String {
    fn from(addr: Addr) -> Self {
        addr.to_string()
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Hash::PREFIX, hex::encode(&self.0))
    }
}

impl fmt::Debug for Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Addr({}{})", Hash::PREFIX, hex::encode(&self.0))
    }
}
