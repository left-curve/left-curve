use {
    crate::{
        call_inner, forward_ref_binop_typed, forward_ref_op_assign_typed, generate_int,
        impl_all_ops_and_assign, impl_assign, impl_base_ops, impl_next, impl_signed_ops, Bytable,
        CheckedOps, Inner, NextNumber, NumberConst, Sqrt, StdError, StdResult,
    },
    bnum::types::{I256, I512, U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Not, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Int<U>(pub(crate) U);

impl<U> Int<U> {
    pub const fn new(value: U) -> Self {
        Self(value)
    }

    pub fn new_from(value: impl Into<U>) -> Self {
        Self(value.into())
    }
}

impl<U> Int<U>
where
    U: Copy,
{
    pub const fn number(self) -> U {
        self.0
    }
}

// --- Const ---
impl<U> Int<U>
where
    U: NumberConst,
{
    pub const MAX: Self = Self(U::MAX);
    pub const MIN: Self = Self(U::MIN);
    pub const ONE: Self = Self(U::ONE);
    pub const TEN: Self = Self(U::TEN);
    pub const ZERO: Self = Self(U::ZERO);
}

// --- Inner ---
impl<U> Inner for Int<U> {
    type U = U;
}

// --- Bytable ---
impl<U, const S: usize> Bytable<S> for Int<U>
where
    U: Bytable<S>,
{
    fn from_be_bytes(data: [u8; S]) -> Self {
        Self(U::from_be_bytes(data))
    }

    fn from_le_bytes(data: [u8; S]) -> Self {
        Self(U::from_le_bytes(data))
    }

    fn to_be_bytes(self) -> [u8; S] {
        self.0.to_be_bytes()
    }

    fn to_le_bytes(self) -> [u8; S] {
        self.0.to_le_bytes()
    }

    fn grow_be_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_be_bytes(data)
    }

    fn grow_le_bytes<const INPUT_SIZE: usize>(data: [u8; INPUT_SIZE]) -> [u8; S] {
        U::grow_le_bytes(data)
    }
}

// --- CheckedOps ---
#[rustfmt::skip]
impl<U> CheckedOps for Int<U>
where
    U: CheckedOps,
{
    call_inner!(fn checked_add,    field 0, => Result<Self>);
    call_inner!(fn checked_sub,    field 0, => Result<Self>);
    call_inner!(fn checked_mul,    field 0, => Result<Self>);
    call_inner!(fn checked_div,    field 0, => Result<Self>);
    call_inner!(fn checked_rem,    field 0, => Result<Self>);
    call_inner!(fn checked_pow,    arg u32, => Result<Self>);
    call_inner!(fn checked_shl,    arg u32, => Result<Self>);
    call_inner!(fn checked_shr,    arg u32, => Result<Self>);
    call_inner!(fn checked_ilog2,           => StdResult<u32>);
    call_inner!(fn checked_ilog10,          => StdResult<u32>);
    call_inner!(fn wrapping_add,   field 0, => Self);
    call_inner!(fn wrapping_sub,   field 0, => Self);
    call_inner!(fn wrapping_mul,   field 0, => Self);
    call_inner!(fn wrapping_pow,   arg u32, => Self);
    call_inner!(fn saturating_add, field 0, => Self);
    call_inner!(fn saturating_sub, field 0, => Self);
    call_inner!(fn saturating_mul, field 0, => Self);
    call_inner!(fn saturating_pow, arg u32, => Self);
    call_inner!(fn abs,                     => Self);
    call_inner!(fn is_zero,                 => bool);
}

// --- Sqrt ---
impl<U> Sqrt for Int<U>
where
    U: Copy + Sqrt,
{
    fn checked_sqrt(self) -> StdResult<Self> {
        self.number().checked_sqrt().map(Self::new)
    }
}

// --- full_mull ---
impl<U> Int<U>
where
    Int<U>: NextNumber,
    <Int<U> as NextNumber>::Next: From<Int<U>> + CheckedOps + ToString,
{
    /// Convert the current [`Int`] to [`NextNumber::Next`]
    ///
    /// Example: [`Uint64`] -> [`Uint128`]
    pub fn as_next(self) -> <Int<U> as NextNumber>::Next {
        <Int<U> as NextNumber>::Next::from(self)
    }

    pub fn checked_full_mul(self, rhs: impl Into<Self>) -> StdResult<<Int<U> as NextNumber>::Next> {
        let s = <Int<U> as NextNumber>::Next::from(self);
        let r = <Int<U> as NextNumber>::Next::from(rhs.into());
        s.checked_mul(r)
    }

    pub fn full_mul(self, rhs: impl Into<Self>) -> <Int<U> as NextNumber>::Next {
        self.checked_full_mul(rhs).unwrap()
    }
}

// --- multiply_ratio ---
impl<U> Int<U>
where
    U: NumberConst + PartialEq,
    Int<U>: NextNumber + CheckedOps + Copy,
    <Int<U> as NextNumber>::Next: From<Int<U>> + CheckedOps + TryInto<Int<U>> + ToString + Clone,
{
    pub fn checked_multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let numerator: Self = numerator.into();
        let denominator: <Int<U> as NextNumber>::Next = Into::<Self>::into(denominator).into();

        let next_result = self.checked_full_mul(numerator)?.checked_div(denominator)?;
        next_result
            .clone()
            .try_into()
            .map_err(|_| StdError::overflow_conversion::<_, Self>(next_result))
    }

    pub fn multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> Self {
        self.checked_multiply_ratio_floor(numerator, denominator)
            .unwrap()
    }

    pub fn checked_multiply_ratio_ceil<A: Into<Self>, B: Into<Self>>(
        self,
        numerator: A,
        denominator: B,
    ) -> StdResult<Self> {
        let numerator: Self = numerator.into();
        let dividend = self.checked_full_mul(numerator)?;
        let floor_result = self.checked_multiply_ratio_floor(numerator, denominator)?;
        let remained = dividend.checked_rem(floor_result.as_next())?;
        if !remained.is_zero() {
            Self::ONE.checked_add(floor_result)
        } else {
            Ok(floor_result)
        }
    }
}

impl_signed_ops!(impl<U> Not for Int<U>);
impl_signed_ops!(impl<U> Neg for Int<U>);

impl_base_ops!(impl<U> Add, add for Int<U> where sub fn checked_add);
impl_base_ops!(impl<U> Sub, sub for Int<U> where sub fn checked_sub);
impl_base_ops!(impl<U> Mul, mul for Int<U> where sub fn checked_mul);
impl_base_ops!(impl<U> Div, div for Int<U> where sub fn checked_div);
impl_base_ops!(impl<U> Shl, shl for Int<U> where sub fn checked_shl, u32);
impl_base_ops!(impl<U> Shr, shr for Int<U> where sub fn checked_shr, u32);

impl_assign!(impl<U> AddAssign, add_assign for Int<U> where sub fn checked_add);
impl_assign!(impl<U> SubAssign, sub_assign for Int<U> where sub fn checked_sub);
impl_assign!(impl<U> MulAssign, mul_assign for Int<U> where sub fn checked_mul);
impl_assign!(impl<U> DivAssign, div_assign for Int<U> where sub fn checked_div);
impl_assign!(impl<U> ShrAssign, shr_assign for Int<U> where sub fn checked_shr, u32);
impl_assign!(impl<U> ShlAssign, shl_assign for Int<U> where sub fn checked_shl, u32);

forward_ref_binop_typed!(impl<U> Add, add for Int<U>, Int<U>);
forward_ref_binop_typed!(impl<U> Sub, sub for Int<U>, Int<U>);
forward_ref_binop_typed!(impl<U> Mul, mul for Int<U>, Int<U>);
forward_ref_binop_typed!(impl<U> Div, div for Int<U>, Int<U>);
forward_ref_binop_typed!(impl<U> Rem, rem for Int<U>, Int<U>);
forward_ref_binop_typed!(impl<U> Shl, shl for Int<U>, u32);
forward_ref_binop_typed!(impl<U> Shr, shr for Int<U>, u32);

forward_ref_op_assign_typed!(impl<U> AddAssign, add_assign for Int<U>, Int<U>);
forward_ref_op_assign_typed!(impl<U> SubAssign, sub_assign for Int<U>, Int<U>);
forward_ref_op_assign_typed!(impl<U> MulAssign, mul_assign for Int<U>, Int<U>);
forward_ref_op_assign_typed!(impl<U> DivAssign, div_assign for Int<U>, Int<U>);
forward_ref_op_assign_typed!(impl<U> ShrAssign, shr_assign for Int<U>, u32);
forward_ref_op_assign_typed!(impl<U> ShlAssign, shl_assign for Int<U>, u32);

impl<U> FromStr for Int<U>
where
    U: FromStr,
    <U as FromStr>::Err: ToString,
{
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U::from_str(s)
            .map(Self)
            .map_err(|err| StdError::parse_number::<Self>(s, err))
    }
}

impl<U> fmt::Display for Int<U>
where
    U: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<U> ser::Serialize for Int<U>
where
    U: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, U> de::Deserialize<'de> for Int<U>
where
    U: Default + FromStr,
    <U as FromStr>::Err: Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(UintVisitor::<U>::default())
    }
}

#[derive(Default)]
struct UintVisitor<U> {
    _marker: PhantomData<U>,
}

impl<'de, U> de::Visitor<'de> for UintVisitor<U>
where
    U: FromStr,
    <U as FromStr>::Err: Display,
{
    type Value = Int<U>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Change this message in base at the type of U
        f.write_str("a string-encoded 256-bit unsigned integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse::<U>().map(Int::<U>).map_err(E::custom)
    }
}

// ------------------------------ concrete types -------------------------------

// Uint64
generate_int!(
    name = Uint64,
    inner_type = u64,
    from_int = [],
    from_std = [u32, u16, u8],
    try_from_int = []
);

// Uint128
generate_int!(
    name = Uint128,
    inner_type = u128,
    from_int = [Uint64],
    from_std = [u32, u16, u8],
    try_from_int = []
);

// Uint256
generate_int!(
    name = Uint256,
    inner_type = U256,
    from_int = [Uint64, Uint128],
    from_std = [u32, u16, u8],
    try_from_int = []
);

// Uint512
generate_int!(
    name = Uint512,
    inner_type = U512,
    from_int = [Uint256, Uint64, Uint128],
    from_std = [u32, u16, u8],
    try_from_int = []
);

// Int64
generate_int!(
    name = Int64,
    inner_type = i64,
    from_int = [],
    from_std = [u32, u16, u8, i32, i16, i8],
    try_from_int = []
);

// Int128
generate_int!(
    name = Int128,
    inner_type = i128,
    from_int = [Int64, Uint64],
    from_std = [u32, u16, u8, i32, i16, i8],
    try_from_int = [Uint128]
);

// Int256
generate_int!(
    name = Int256,
    inner_type = I256,
    from_int = [Int64, Int128, Uint64, Uint128],
    from_std = [u32, u16, u8, i32, i16, i8],
    try_from_int = [Uint256]
);

// Int512
generate_int!(
    name = Int512,
    inner_type = I512,
    from_int = [Int64, Int128, Int256, Uint64, Uint128, Uint256],
    from_std = [u32, u16, u8, i32, i16, i8],
    try_from_int = [Uint512]
);

// Implementations of [`Next`] has to be done after all the types are defined.
impl_next!(Uint64, Uint128);
impl_next!(Uint128, Uint256);
impl_next!(Uint256, Uint512);
impl_next!(Int64, Int128);
impl_next!(Int128, Int256);
impl_next!(Int256, Int512);

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{grow_be_int, grow_be_uint, grow_le_int, grow_le_uint},
        bnum::types::{I256, I512, U256},
    };

    #[test]
    fn t1() {
        let a = (-2_i128).to_be_bytes();
        let a = grow_be_int::<16, 32>(a);
        let a = I256::from_be_bytes(a);
        println!("{:?}", a);

        let a = (-2_i128).to_le_bytes();
        let a = grow_le_int::<16, 32>(a);
        let a = I256::from_le_bytes(a);
        println!("{:?}", a);

        let a = (2_u128).to_be_bytes();
        let a = grow_be_uint::<16, 32>(a);
        let a = U256::from_be_bytes(a);
        println!("{:?}", a);

        let a = (2_i128).to_le_bytes();
        let a = grow_le_uint::<16, 32>(a);
        let a = I256::from_le_bytes(a);
        println!("{:?}", a);
    }

    #[test]
    fn t2_conversion() {
        let foo = Int128::new(1);
        assert_eq!(foo.as_next(), Int256::new(I256::ONE));

        let foo = Int256::ONE;
        assert_eq!(foo.as_next(), Int512::new(I512::ONE));

        let foo = Int128::new(10);
        assert_eq!(TryInto::<Uint128>::try_into(foo).unwrap(), Uint128::new(10));

        let foo = Int128::new(-10);
        TryInto::<Uint128>::try_into(foo).unwrap_err();

        let foo = Int128::new(10);
        assert_eq!(TryInto::<u128>::try_into(foo).unwrap(), 10);

        let foo = Int128::new(-10);
        TryInto::<u128>::try_into(foo).unwrap_err();
    }

    #[test]
    fn t3_ops() {
        let foo = Uint64::new(1);
        let bar = Uint128::new(2);
        assert_eq!(bar + foo, Uint128::new(3));

        let foo = Int64::new(1);
        assert_eq!(-foo, Int64::new(-1));

        let foo = Uint64::new(10);
        let bar = Int128::new(-10);
        assert_eq!(bar + foo, Int128::new(0));
        assert_eq!(foo + bar, Int128::new(0));

        let foo = Uint64::new(10);
        let bar = Uint128::new(20);
        let zzz = Uint64::new(20);
        assert_eq!(foo + zzz, Uint64::new(30));

        // Check if additions between an owned value and a reference works.
        // Clippy would complain "needless reference" here, so disable it.
        #[allow(clippy::op_ref)]
        {
            assert_eq!(foo + &zzz, Uint64::new(30));
            assert_eq!(&foo + zzz, Uint64::new(30));
            assert_eq!(&foo + &zzz, Uint64::new(30));
            assert_eq!(foo + &bar, Uint128::new(30));
            assert_eq!(bar + &foo, Uint128::new(30));
        }

        let mut foo = Uint128::new(10);
        foo += 10_u32;
        assert_eq!(foo, Uint128::new(20));
    }
}
