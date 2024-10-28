use {
    grug::{Binary, Hash256, HashExt, Inner, JsonDeExt},
    serde::{de::Visitor, Deserialize},
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {}

#[grug::derive(Serde)]
pub enum ExecuteMsg {}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {}

#[derive(Debug)]
pub struct VAA {
    pub guardian_set_index: u32,
    pub signatures: Vec<[u8; VAA::SIGNATURE_LEN]>,
    pub hash: Hash256,
    pub timestamp: u32,
    pub nonce: u32,
    pub emitter_chain: u16,
    pub emitter_address: [u8; 32],
    pub sequence: u32,
    pub consistency_level: u8,
    pub payload: Vec<u8>,
}

impl VAA {
    pub const HEADER_LEN: usize = 6;
    pub const SIGNATURE_LEN: usize = 66;
}

impl<'de> Deserialize<'de> for VAA {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(VAAVisitor {})
    }
}

pub struct VAAVisitor;

impl<'de> Visitor<'de> for VAAVisitor {
    type Value = VAA;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("vaa")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut bytes = format!("\"{v}\"")
            .deserialize_json::<Binary>()
            .unwrap()
            .into_inner();
        let guardian_set_index = bytes.next_u32();
        let len_signers = bytes.next_u8();

        let signs = [..len_signers]
            .iter()
            .map(|_| bytes.next_bytes::<{ VAA::SIGNATURE_LEN }>())
            .collect::<Vec<_>>();

        // We should use api functions but we are inside a trait, can't use it.
        // For now use the HashExt trait directly.
        // This need double hash
        let hash = bytes.hash256().keccak256().keccak256();

        let timestamp = bytes.next_u32();
        let nonce = bytes.next_u32();
        let emitter_chain = bytes.next_u16();

        let emitter_address = bytes.next_bytes::<32>();
        let sequence = bytes.next_u32();
        let consistency_level = bytes.next_u8();

        Ok(VAA {
            guardian_set_index,
            signatures: signs,
            hash,
            timestamp,
            nonce,
            emitter_chain,
            emitter_address,
            sequence,
            consistency_level,
            payload: bytes,
        })
    }
}

pub trait Bytes {
    fn next_u8(&mut self) -> u8;
    fn next_u16(&mut self) -> u16;
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
    fn next_bytes<const S: usize>(&mut self) -> [u8; S];
}

macro_rules! impl_bytes {
    ($($n:ty => $b:expr),+ ) => {
        paste::paste! {
            $(fn [<next_ $n>](&mut self) -> $n {
                let mut bytes: [u8; $b] = [0; $b];
                bytes.copy_from_slice(&self[..$b]);
                self.drain(..$b);
                (<$n>::from_be_bytes(bytes))
            })*
        }
    };
}

impl Bytes for Vec<u8> {
    impl_bytes!( u8 => 1, u16 => 2, u32 => 4, u64 => 8);

    fn next_bytes<const S: usize>(&mut self) -> [u8; S] {
        let mut bytes: [u8; S] = [0; S];
        bytes.copy_from_slice(&self[..S]);
        self.drain(..S);
        bytes
    }
}

#[test]
fn des_vaa() {
    let str = r#""UE5BVQEAAAADuAEAAAAEDQB4sfOiNVFLGzP9vWUtLL9xo0oJexwuiQMG6CMSqeFPWQJQZ7ReMq06+fmBKOik0zH3iWSfoFUiRLojvcUy26rfAAIfuODXyE9VywWQUdXqBTR2YNoSYWjmmqht5oMIaUXedlAWv68duEneRk4a+ydueQo+ETXoF2TyLtcGlOUuCULOAQMc0kxsV+RpDkGktFtGmARBDIQhB96gycUmcNK2mOMgQEGGHz9p71vDNina8OqjkOVEmDuCNmlo23SpBCFEnrNiAQTDam5pOrQiugH+PK7jXIWHQJ81jxi5YUXVHhdo/1nsGyuw1AkUAd5tpt2/twElAwZ5lzUR5NnmK3eVFHMxuC5VAAbb545Np1ROTVsCd4S7ZUsY+a9eedsoULdvrCexT+7vvhiERLC0FKSysPuMwkdcYHMqhAouijk1m0LrmtwUWIEVAQo5RTwleVvZ1JYadGhSvdhsjDW9S78iLm1uI0R+rHYdLwy7sEZo0g3ZSVpWBYlAPn1wLVMzeEwqAJEBoVApi+NpAAut5kTXFsZpkkXh5IzPyPRQ1husOADT9StHx+cvrazYoxfWN1cdtZ9hfNDUPb2e7RqDMkKl9S7dAyTuL1eWFIhGAQzgTUNjP9s7ZCwZGjPQEUA4gDnLAiWKIBnOIv8wmzNjmGOe6vVNMsaFTswxhoBPdHHx1dDnNvLfaSbqUvj/Wk3dAQ06VyC/93Jdr5bt3BRRvG1Jv8q0KP2byzQ7QBkY0pavqVSPBNUujh7152LaJ+V6SYsYuKCcPp8yX5Etl7Ua83AkAQ6pS7MgwaDYKDXivSywfAG/663qvkKSgcmEdPuvUVcaF1vi+z0MMt5EasSwhdhqIQ0TjPnXdg3l31Hg7DEshTVhAQ+VtnxjhhCnNJaPnkJn4TBZ5/Z95IFUQ/X1g2sHhISkYQTZeMRJUad9kWfoLYnWt+Mi0/u92nNjndZX4okGc9fvABEwEwIIIf3F0GYWYNNlIak/abrv/BRm6tmCSHnkQE+EfSOb9eUGT0k8KU/AffpqSKIIBV1ZH99TxibgK/w/V0yqARIu0qUy13udPIAzNYTexgFuPGvMQMI7Y3/26jTUWGr8GFGU8K27ibEWJ4/De3w31KFuI1nZcCCNFzkwAXhPJ+uYAWcfwiEAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVl9wAUFVV1YAAAAAAApjHbEAACcQM7UOCZfNwVK88/n5iz5gfKIp+nwBAFUA/2FJGpMREt3xvYFHzRtkE3X3n1glEm1mVICHRjT9Cs4AAAA6WDkPmgAAAAAMxnHg////+AAAAABnH8IhAAAAAGcfwiEAAAA6daKuuAAAAAANl8w+CkEwqUwIseLK3fPQfxQIdRO3+UgGZbXm03BkQ7m6R/YIl0bZ7NhjjqiptcAFAtwqcVmEZO7pYpfZM1J1sx8Dvs/1PJ68AI4C6i9ePx7qOMPxSBnjT09SWyLTTf1xxiTrIZfcy8FE5tOD1MTcx253Uyo73MNpHyw0pLgeWRYmomFEu6imVgCKx5kr3bX9Y2+I+JGM4n/I4dTlNJGws0pdtvg9jI4g4RU3Hcx0vVzJIWB5iN/cfjD/IaMk1SfIwqPDF/I06m4PtyoF""#;
    serde_json::from_str::<VAA>(str).unwrap();
}
