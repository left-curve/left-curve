use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// When serializing to JSON, gives an pair of brackets: `{}`.
/// When serializing with Borsh, gives empty bytes: ``.
/// Useful for use in contract messages when there isn't any intended inputs, or
/// in contract storage to represent empty value (e.g. in `grug::Set`).
#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use {
        crate::{BorshExt, Empty, JsonExt},
        serde_json::json,
    };

    #[test]
    fn encoding_with_serde() {
        let empty_json = json!({});
        assert_eq!(Empty {}.to_json_value().unwrap(), empty_json);
        assert_eq!(Empty::from_json_value(empty_json).unwrap(), Empty {});
    }

    #[test]
    fn encoding_with_borsh() {
        assert!(Empty {}.to_borsh_vec().unwrap().is_empty());
        assert_eq!(Empty::from_borsh_slice([]).unwrap(), Empty {});
    }
}
