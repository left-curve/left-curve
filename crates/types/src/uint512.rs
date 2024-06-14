use {
    crate::{StdError, StdResult, Uint256},
    bnum::types::U512,
    std::{
        fmt, mem,
        ops::{Div, Mul},
    },
};

/// This is only used internally to implement the `checked_multiply_ratio` method
/// for Uint256 and Decimal256. Therefore features are lacking compared to other
/// Uint types.
///
/// I don't think there's any practical use for 512-bit numbers in DeFi (I don't
/// even think there's need for 256-bit numbers, but Ethereum uses it so we have
/// to support it - we may have to deal with bridged tokens from Eth, for example).
/// If you're in need of a full feature Uint512 please reach out, we will work on it.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uint512(U512);

impl Uint512 {
    pub fn checked_mul(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_mul(other.0)
            .map(Self)
            .ok_or_else(|| StdError::overflow_mul(self, other))
    }

    pub fn checked_div(self, other: Self) -> StdResult<Self> {
        self.0
            .checked_div(other.0)
            .map(Self)
            .ok_or_else(|| StdError::division_by_zero(self))
    }
}

impl Mul for Uint512 {
    type Output = Self;

    fn mul(self, rhs: Uint512) -> Self::Output {
        self.checked_mul(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl Div for Uint512 {
    type Output = Self;

    fn div(self, rhs: Uint512) -> Self::Output {
        self.checked_div(rhs).unwrap_or_else(|err| panic!("{err}"))
    }
}

impl From<Uint256> for Uint512 {
    fn from(value: Uint256) -> Self {
        let bytes = value.to_le_bytes();
        Self(U512::from_digits([
            u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            u64::from_le_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
            u64::from_le_bytes([
                bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30],
                bytes[31],
            ]),
            0,
            0,
            0,
            0,
        ]))
    }
}

impl TryFrom<Uint512> for Uint256 {
    type Error = StdError;

    fn try_from(value: Uint512) -> StdResult<Self> {
        let words = value.0.digits();

        let higher = [
            words[4].to_le_bytes(),
            words[5].to_le_bytes(),
            words[6].to_le_bytes(),
            words[7].to_le_bytes(),
        ];
        if higher != [[0; 8]; 4] {
            return Err(StdError::overflow_conversion::<_, Uint256>(value));
        }

        let lower = [
            words[0].to_le_bytes(),
            words[1].to_le_bytes(),
            words[2].to_le_bytes(),
            words[3].to_le_bytes(),
        ];
        let lower = unsafe { mem::transmute::<[[u8; 8]; 4], [u8; 32]>(lower) };

        Ok(Uint256::from_le_bytes(lower))
    }
}

impl fmt::Display for Uint512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}
