use {
    crate::LeEcdsaMessage,
    anyhow::{Result, bail},
    grug::{AddrEncoder, Binary, EncodedBytes, NonEmpty},
};

pub type PythId = EncodedBytes<[u8; 32], AddrEncoder>;

pub type PythLazerId = u32;

#[grug::derive(Serde)]
pub struct LatestVaaResponse {
    pub binary: LatestVaaBinaryResponse,
}

#[grug::derive(Serde)]
pub struct LatestVaaBinaryResponse {
    pub data: Vec<Binary>,
}

#[grug::derive(Serde)]
pub enum PriceUpdate {
    Core(NonEmpty<Vec<Binary>>),
    Lazer(LeEcdsaMessage),
}

impl PriceUpdate {
    /// Check if the `PriceUpdate` is a Core.
    pub fn is_core(&self) -> bool {
        matches!(self, PriceUpdate::Core(_))
    }

    /// Check if the `PriceUpdate` is a Lazer.
    pub fn is_lazer(&self) -> bool {
        matches!(self, PriceUpdate::Lazer(_))
    }

    /// Try to cast `PriceUpdate` to `Core`.
    pub fn try_into_core(&self) -> Result<NonEmpty<Vec<Binary>>> {
        match self {
            PriceUpdate::Core(core) => Ok(core.clone()),
            _ => bail!("PriceUpdate is not Core"),
        }
    }

    /// Try to cast `PriceUpdate` to `Lazer`.
    pub fn try_into_lazer(&self) -> Result<LeEcdsaMessage> {
        match self {
            PriceUpdate::Lazer(lazer) => Ok(lazer.clone()),
            _ => bail!("PriceUpdate is not Lazer"),
        }
    }
}
