use {
    crate::{
        call_inner, forward_ref_binop_typed, forward_ref_op_assign_typed, generate_int,
        impl_all_ops_and_assign, impl_assign_integer, impl_assign_number, impl_integer, impl_next,
        impl_number, Bytable, Inner, Integer, MultiplyRatio, NextNumber, Number, NumberConst, Sign,
        StdError, StdResult,
    },
    bnum::types::{U256, U512},
    borsh::{BorshDeserialize, BorshSerialize},
    forward_ref::{forward_ref_binop, forward_ref_op_assign},
    serde::{de, ser},
    std::{
        fmt::{self, Display},
        marker::PhantomData,
        ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
        str::FromStr,
    },
};

// ------------------------------- generic type --------------------------------

#[derive(
    BorshSerialize, BorshDeserialize, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Int<U>(pub(crate) U);

// --- Init ---
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

// --- Sign ---
impl<U> Sign for Int<U> {
    fn is_positive(self) -> bool {
        true
    }
}

// --- Constants ---
impl<U> NumberConst for Int<U>
where
    U: NumberConst,
{
    const MAX: Self = Self(U::MAX);
    const MIN: Self = Self(U::MIN);
    const ONE: Self = Self(U::ONE);
    const TEN: Self = Self(U::TEN);
    const ZERO: Self = Self(U::ZERO);
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

// --- Number ---
#[rustfmt::skip]
impl<U> Number for Int<U>
where
    U: Number,
{
    call_inner!(fn checked_add,    field 0, => Result<Self>);
    call_inner!(fn checked_sub,    field 0, => Result<Self>);
    call_inner!(fn checked_mul,    field 0, => Result<Self>);
    call_inner!(fn checked_div,    field 0, => Result<Self>);
    call_inner!(fn checked_rem,    field 0, => Result<Self>);
    call_inner!(fn checked_pow,    arg u32, => Result<Self>);
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
    call_inner!(fn checked_sqrt,            => Result<Self>);

}

// --- Integer ---
#[rustfmt::skip]
impl<U> Integer for Int<U>
where
    U: Integer
    {
    call_inner!(fn checked_shl, arg u32, => Result<Self>);
    call_inner!(fn checked_shr, arg u32, => Result<Self>);
    call_inner!(fn checked_ilog2,        => StdResult<u32>);
    call_inner!(fn checked_ilog10,       => StdResult<u32>);
    }

// --- full_mull ---
impl<U> Int<U>
where
    Int<U>: NextNumber,
    <Int<U> as NextNumber>::Next: Number + ToString,
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
impl<U> MultiplyRatio for Int<U>
where
    U: NumberConst + PartialEq,
    Int<U>: NextNumber + Number + Copy,
    <Int<U> as NextNumber>::Next: Number + ToString + Clone,
{
    fn checked_multiply_ratio_floor<A: Into<Self>, B: Into<Self>>(
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

    fn checked_multiply_ratio_ceil<A: Into<Self>, B: Into<Self>>(
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

// --- FromStr ---
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

// --- Display ---
impl<U> fmt::Display for Int<U>
where
    U: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// --- serde::Serialize ---
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

// --- serde::Deserialize ---
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

impl_number!(impl<U> Add, add for Int<U> where sub fn checked_add);
impl_number!(impl<U> Sub, sub for Int<U> where sub fn checked_sub);
impl_number!(impl<U> Mul, mul for Int<U> where sub fn checked_mul);
impl_number!(impl<U> Div, div for Int<U> where sub fn checked_div);
impl_integer!(impl<U> Shl, shl for Int<U> where sub fn checked_shl, u32);
impl_integer!(impl<U> Shr, shr for Int<U> where sub fn checked_shr, u32);

impl_assign_number!(impl<U> AddAssign, add_assign for Int<U> where sub fn checked_add);
impl_assign_number!(impl<U> SubAssign, sub_assign for Int<U> where sub fn checked_sub);
impl_assign_number!(impl<U> MulAssign, mul_assign for Int<U> where sub fn checked_mul);
impl_assign_number!(impl<U> DivAssign, div_assign for Int<U> where sub fn checked_div);
impl_assign_integer!(impl<U> ShrAssign, shr_assign for Int<U> where sub fn checked_shr, u32);
impl_assign_integer!(impl<U> ShlAssign, shl_assign for Int<U> where sub fn checked_shl, u32);

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

// ------------------------------ concrete types -------------------------------

// Uint64
generate_int!(
    name = Uint64,
    inner_type = u64,
    from_int = [],
    from_std = [u32, u16, u8],
);

// Uint128
generate_int!(
    name = Uint128,
    inner_type = u128,
    from_int = [Uint64],
    from_std = [u32, u16, u8],
);

// Uint256
generate_int!(
    name = Uint256,
    inner_type = U256,
    from_int = [Uint64, Uint128],
    from_std = [u32, u16, u8],
);

// Uint512
generate_int!(
    name = Uint512,
    inner_type = U512,
    from_int = [Uint256, Uint64, Uint128],
    from_std = [u32, u16, u8],
);

// Implementations of [`Next`] has to be done after all the types are defined.
impl_next!(Uint64, Uint128);
impl_next!(Uint128, Uint256);
impl_next!(Uint256, Uint512);

#[cfg(test)]
mod test {
    use {
        crate::{Int, Number, Uint128, Uint256},
        paste::paste,
        std::{fmt::Debug, str::FromStr},
        test_case::test_case,
    };

    // 1: Example of wrapping test inside macro.
    // This has the best flexibility but is harder to read.
    macro_rules! base_math {
        ($x:expr, $y:expr, $tt:tt, $id:literal) => {
            paste! {
                #[test]
                fn [<$id>]() {
                    assert_eq!($x + $y, $tt::from_str("30").unwrap());

                }
            }
        };
    }

    base_math!(
        Uint128::new(20),
        Uint128::new(10),
        Uint128,
        "uint_base_128_1"
    );
    base_math!(
        Uint256::new(20_u128.into()),
        Uint256::new(10_u128.into()),
        Uint256,
        "uint_base_256_1"
    );

    // 2: TestCase.
    // This is the most readable way to write tests, but require to define typing.
    // Is the most limitated one.
    #[test_case(Uint128::new(20), Uint128::new(10) ; "uint_base_128_2")]
    #[test_case(Uint256::new(20_u128.into()), Uint256::new(10_u128.into()) ; "uint_base_256_2")]
    fn base_ops<X>(x: Int<X>, y: Int<X>)
    where
        Int<X>: Number + FromStr + PartialEq + Debug,
        <Int<X> as std::str::FromStr>::Err: Debug,
    {
        assert_eq!(x + y, Int::<X>::from_str("30").unwrap());
    }

    // 3: grug_test_case.
    // With only one macro, is possible to define multiple tests.
    // On the macro call is possible to define the body of the test.
    // The main limitation is that i've not found a way to properly assery the result.
    // for example assert_eq!(x + y, Int::new(30)) is not possible because the closure has no knowledge of the type of Int.;
    // is there a way to do it?
    macro_rules! grug_test_case {
        (
            $(
                [$($param_value:expr),+ ; $fn_name:ident]
            ),*,
            $body:block
        ) => {
            $(
                #[test]
                fn $fn_name() {
                    ($body)($($param_value),*);
                }
            )*
        };
    }

    grug_test_case!(
        [Uint128::new(20),             Uint128::new(10)            ; test1 ],
        [Uint256::new(20_u128.into()), Uint256::new(10_u128.into()); test2 ],
        {|x, y| {
            let _ = x - y;
        }}
    );
}
