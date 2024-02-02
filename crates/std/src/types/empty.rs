use serde::{Deserialize, Serialize};

/// When serializing to JSON, gives an pair of brackets: `{}`. Useful for use in
/// contract messages when there isn't any intended inputs.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{from_json, to_json},
    };

    #[test]
    fn serializing_empty() {
        assert_eq!(to_json(&Empty {}).unwrap(), b"{}".to_vec().into());
        assert_eq!(from_json::<Empty>(b"{}").unwrap(), Empty {});
    }
}
