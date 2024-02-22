use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// When serializing to JSON, gives an pair of brackets: `{}`.
/// When serializing with Borsh, gives empty bytes: ``.
/// Useful for use in contract messages when there isn't any intended inputs, or
/// in contract storage to represent empty value (e.g. in `cw_std::Set`).
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_borsh, from_json, to_borsh, to_json},
    };

    #[test]
    fn encoding_with_serde() {
        assert_eq!(to_json(&Empty {}).unwrap(), b"{}".to_vec().into());
        assert_eq!(from_json::<Empty>(b"{}").unwrap(), Empty {});
    }

    #[test]
    fn encoding_with_borsh() {
        assert_eq!(to_borsh(&Empty {}).unwrap(), b"".to_vec().into());
        assert_eq!(from_borsh::<Empty>(b"").unwrap(), Empty {});
    }
}
