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
        crate::{from_borsh_slice, from_json_value, to_borsh_vec, to_json_value},
        serde_json::json,
    };

    #[test]
    fn encoding_with_serde() {
        let empty_json = json!({});
        assert_eq!(to_json_value(&Empty {}).unwrap(), empty_json);
        assert_eq!(from_json_value::<Empty>(empty_json).unwrap(), Empty {});
    }

    #[test]
    fn encoding_with_borsh() {
        assert!(to_borsh_vec(&Empty {}).unwrap().is_empty());
        assert_eq!(from_borsh_slice::<Empty>(&[]).unwrap(), Empty {});
    }
}
