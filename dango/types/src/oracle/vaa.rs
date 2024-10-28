use {
    grug::{Binary, Hash256, HashExt, Inner, StdError, StdResult},
    serde::{de::Visitor, Deserialize},
    std::{ops::Deref, str::FromStr},
};

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
        || -> StdResult<VAA> {
            let mut bytes = BytesAnalyzer::new(Binary::from_str(v)?.into_inner());

            let guardian_set_index = bytes.next_u32()?;
            let len_signers = bytes.next_u8();

            let signs = [..len_signers]
                .iter()
                .map(|_| bytes.next_bytes::<{ VAA::SIGNATURE_LEN }>())
                .collect::<StdResult<Vec<_>>>()?;

            // We should use api functions but we are inside a trait, can't use it.
            // For now use the HashExt trait directly.
            // This need double hash
            let hash = bytes.deref().hash256().keccak256().keccak256();

            let timestamp = bytes.next_u32()?;
            let nonce = bytes.next_u32()?;
            let emitter_chain = bytes.next_u16()?;

            let emitter_address = bytes.next_bytes::<32>()?;
            let sequence = bytes.next_u32()?;
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
                payload: bytes.consume(),
            })
        }()
        .map_err(E::custom)
    }
}

pub struct BytesAnalyzer {
    bytes: Vec<u8>,
    index: usize,
}

macro_rules! impl_bytes {
    ($($n:ty => $size:expr),+ ) => {
        paste::paste! {
            $(pub fn [<next_ $n>](&mut self) -> StdResult<$n> {
                if self.index + $size <= self.bytes.len() {
                    let bytes = &self.bytes[self.index..self.index + $size];
                    self.index += $size;
                    Ok(<$n>::from_be_bytes(bytes.try_into()?))
                } else {
                    Err(StdError::host("Not enough bytes".to_string()))
                }
            })*
        }
    };
}

impl BytesAnalyzer {
    impl_bytes!(u16 => 2, u32 => 4, u64 => 8);

    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, index: 0 }
    }

    pub fn next_u8(&mut self) -> u8 {
        self.index += 1;
        self.bytes[self.index - 1]
    }

    fn next_bytes<const S: usize>(&mut self) -> StdResult<[u8; S]> {
        if self.index + S <= self.bytes.len() {
            let mut bytes: [u8; S] = [0; S];
            bytes.copy_from_slice(&self.bytes[self.index..self.index + S]);
            self.index += S;
            Ok(bytes)
        } else {
            Err(StdError::host("Not enough bytes".to_string()))
        }
    }

    fn consume(mut self) -> Vec<u8> {
        self.bytes.split_off(self.index)
    }
}

impl Deref for BytesAnalyzer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes[self.index..]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn byte_analizer() {
        let raw = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

        let mut analizer = BytesAnalyzer::new(raw);

        assert_eq!(analizer.next_u8(), 1);
        assert_eq!(analizer.next_u16().unwrap(), u16::from_be_bytes([2, 3]));
        assert_eq!(
            analizer.next_u32().unwrap(),
            u32::from_be_bytes([4, 5, 6, 7])
        );

        // deref
        assert_eq!(analizer.deref(), &[8, 9, 10, 11, 12, 13, 14, 15]);

        assert_eq!(analizer.next_bytes::<4>().unwrap(), [8, 9, 10, 11]);
        assert_eq!(analizer.consume(), vec![12, 13, 14, 15]);
    }

    #[test]
    fn des_vaa() {
        let str = r#""UE5BVQEAAAADuAEAAAAEDQB4sfOiNVFLGzP9vWUtLL9xo0oJexwuiQMG6CMSqeFPWQJQZ7ReMq06+fmBKOik0zH3iWSfoFUiRLojvcUy26rfAAIfuODXyE9VywWQUdXqBTR2YNoSYWjmmqht5oMIaUXedlAWv68duEneRk4a+ydueQo+ETXoF2TyLtcGlOUuCULOAQMc0kxsV+RpDkGktFtGmARBDIQhB96gycUmcNK2mOMgQEGGHz9p71vDNina8OqjkOVEmDuCNmlo23SpBCFEnrNiAQTDam5pOrQiugH+PK7jXIWHQJ81jxi5YUXVHhdo/1nsGyuw1AkUAd5tpt2/twElAwZ5lzUR5NnmK3eVFHMxuC5VAAbb545Np1ROTVsCd4S7ZUsY+a9eedsoULdvrCexT+7vvhiERLC0FKSysPuMwkdcYHMqhAouijk1m0LrmtwUWIEVAQo5RTwleVvZ1JYadGhSvdhsjDW9S78iLm1uI0R+rHYdLwy7sEZo0g3ZSVpWBYlAPn1wLVMzeEwqAJEBoVApi+NpAAut5kTXFsZpkkXh5IzPyPRQ1husOADT9StHx+cvrazYoxfWN1cdtZ9hfNDUPb2e7RqDMkKl9S7dAyTuL1eWFIhGAQzgTUNjP9s7ZCwZGjPQEUA4gDnLAiWKIBnOIv8wmzNjmGOe6vVNMsaFTswxhoBPdHHx1dDnNvLfaSbqUvj/Wk3dAQ06VyC/93Jdr5bt3BRRvG1Jv8q0KP2byzQ7QBkY0pavqVSPBNUujh7152LaJ+V6SYsYuKCcPp8yX5Etl7Ua83AkAQ6pS7MgwaDYKDXivSywfAG/663qvkKSgcmEdPuvUVcaF1vi+z0MMt5EasSwhdhqIQ0TjPnXdg3l31Hg7DEshTVhAQ+VtnxjhhCnNJaPnkJn4TBZ5/Z95IFUQ/X1g2sHhISkYQTZeMRJUad9kWfoLYnWt+Mi0/u92nNjndZX4okGc9fvABEwEwIIIf3F0GYWYNNlIak/abrv/BRm6tmCSHnkQE+EfSOb9eUGT0k8KU/AffpqSKIIBV1ZH99TxibgK/w/V0yqARIu0qUy13udPIAzNYTexgFuPGvMQMI7Y3/26jTUWGr8GFGU8K27ibEWJ4/De3w31KFuI1nZcCCNFzkwAXhPJ+uYAWcfwiEAAAAAABrhAfrtrFhR4yubI7X5QRqMK6xKrj7U3XuBHdGnLqSqcQAAAAAFVl9wAUFVV1YAAAAAAApjHbEAACcQM7UOCZfNwVK88/n5iz5gfKIp+nwBAFUA/2FJGpMREt3xvYFHzRtkE3X3n1glEm1mVICHRjT9Cs4AAAA6WDkPmgAAAAAMxnHg////+AAAAABnH8IhAAAAAGcfwiEAAAA6daKuuAAAAAANl8w+CkEwqUwIseLK3fPQfxQIdRO3+UgGZbXm03BkQ7m6R/YIl0bZ7NhjjqiptcAFAtwqcVmEZO7pYpfZM1J1sx8Dvs/1PJ68AI4C6i9ePx7qOMPxSBnjT09SWyLTTf1xxiTrIZfcy8FE5tOD1MTcx253Uyo73MNpHyw0pLgeWRYmomFEu6imVgCKx5kr3bX9Y2+I+JGM4n/I4dTlNJGws0pdtvg9jI4g4RU3Hcx0vVzJIWB5iN/cfjD/IaMk1SfIwqPDF/I06m4PtyoF""#;
        serde_json::from_str::<VAA>(str).unwrap();
    }
}
