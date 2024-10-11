use {
    crate::{Binary, MaxLength, StdError},
    std::str::FromStr,
};

pub const MAX_SALT_LENGTH: usize = 70;

pub type Salt = MaxLength<Binary, MAX_SALT_LENGTH>;

impl FromStr for Salt {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.as_bytes().into())
    }
}
