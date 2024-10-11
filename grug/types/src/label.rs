use {
    crate::{MaxLength, StdError},
    std::str::FromStr,
};

pub const MAX_LABEL_LENGTH: usize = 70;

pub type Label = MaxLength<String, MAX_LABEL_LENGTH>;

impl FromStr for Label {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}
