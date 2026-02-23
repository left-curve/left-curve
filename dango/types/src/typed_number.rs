//! A Rust type system that encapsulates the dimensionality of values.

use {
    grug::{Dec128_6, MathResult, Number as _},
    std::{fmt, marker::PhantomData},
};

/// A signed, fixed-point decimal number with three dimensions typically used in
/// financial settings: quantity, USD value, and time duration.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number<Q, U, D> {
    inner: Dec128_6,
    _quantity: PhantomData<Q>,
    _usd: PhantomData<U>,
    _duration: PhantomData<D>,
}

impl<Q, U, D> Number<Q, U, D> {
    pub const fn new(inner: Dec128_6) -> Self {
        Self {
            inner,
            _quantity: PhantomData,
            _usd: PhantomData,
            _duration: PhantomData,
        }
    }

    pub const fn new_int(int: i128) -> Self {
        Self::new(Dec128_6::new(int))
    }

    pub fn checked_add(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_add(rhs.inner).map(Self::new)
    }

    pub fn checked_sub(self, rhs: Self) -> MathResult<Self> {
        self.inner.checked_sub(rhs.inner).map(Self::new)
    }

    pub fn checked_mul<Q1, U1, D1>(
        self,
        rhs: Number<Q1, U1, D1>,
    ) -> MathResult<
        Number<
            <(Q, Q1) as TypeAdd>::Output,
            <(U, U1) as TypeAdd>::Output,
            <(D, D1) as TypeAdd>::Output,
        >,
    >
    where
        (Q, Q1): TypeAdd,
        (U, U1): TypeAdd,
        (D, D1): TypeAdd,
    {
        self.inner.checked_mul(rhs.inner).map(Number::new)
    }

    pub fn checked_div<Q1, U1, D1>(
        self,
        rhs: Number<Q1, U1, D1>,
    ) -> MathResult<
        Number<
            <(Q, Q1) as TypeSub>::Output,
            <(U, U1) as TypeSub>::Output,
            <(D, D1) as TypeSub>::Output,
        >,
    >
    where
        (Q, Q1): TypeSub,
        (U, U1): TypeSub,
        (D, D1): TypeSub,
    {
        self.inner.checked_div(rhs.inner).map(Number::new)
    }
}

impl<Q, U, D> fmt::Display for Number<Q, U, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Represents zero at the type level.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Zero;

/// Represents the sucessor of a type.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Succ<T>(PhantomData<T>);

/// Represents the predecessor of a type.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pred<T>(PhantomData<T>);

/// Describes when two values are multiplied, how their types should be added.
pub trait TypeAdd {
    type Output;
}

impl TypeAdd for (Zero, Zero) {
    type Output = Zero;
}

impl<T> TypeAdd for (Zero, Succ<T>) {
    type Output = Succ<T>;
}

impl<T> TypeAdd for (Zero, Pred<T>) {
    type Output = Pred<T>;
}

impl<T> TypeAdd for (Succ<T>, Zero) {
    type Output = Succ<T>;
}

impl<T> TypeAdd for (Pred<T>, Zero) {
    type Output = Pred<T>;
}

impl<T, U> TypeAdd for (Succ<T>, Succ<U>)
where
    (T, U): TypeAdd,
{
    type Output = Succ<Succ<<(T, U) as TypeAdd>::Output>>;
}

impl<T, U> TypeAdd for (Succ<T>, Pred<U>)
where
    (T, U): TypeAdd,
{
    type Output = <(T, U) as TypeAdd>::Output;
}

impl<T, U> TypeAdd for (Pred<T>, Succ<U>)
where
    (T, U): TypeAdd,
{
    type Output = <(T, U) as TypeAdd>::Output;
}

impl<T, U> TypeAdd for (Pred<T>, Pred<U>)
where
    (T, U): TypeAdd,
{
    type Output = Pred<Pred<<(T, U) as TypeAdd>::Output>>;
}

/// Describes when two values are divided, how their types should be subtracted.
pub trait TypeSub {
    type Output;
}

impl TypeSub for (Zero, Zero) {
    type Output = Zero;
}

impl<T> TypeSub for (Succ<T>, Zero) {
    type Output = Succ<T>;
}

impl<T> TypeSub for (Pred<T>, Zero) {
    type Output = Pred<T>;
}

impl<T> TypeSub for (Zero, Succ<T>)
where
    (Zero, T): TypeSub,
{
    type Output = Pred<<(Zero, T) as TypeSub>::Output>;
}

impl<T> TypeSub for (Zero, Pred<T>)
where
    (Zero, T): TypeSub,
{
    type Output = Succ<<(Zero, T) as TypeSub>::Output>;
}

impl<T, U> TypeSub for (Succ<T>, Succ<U>)
where
    (T, U): TypeSub,
{
    type Output = <(T, U) as TypeSub>::Output;
}

impl<T, U> TypeSub for (Pred<T>, Pred<U>)
where
    (T, U): TypeSub,
{
    type Output = <(T, U) as TypeSub>::Output;
}

impl<T, U> TypeSub for (Succ<T>, Pred<U>)
where
    (T, U): TypeSub,
{
    type Output = Succ<Succ<<(T, U) as TypeSub>::Output>>;
}

impl<T, U> TypeSub for (Pred<T>, Succ<U>)
where
    (T, U): TypeSub,
{
    type Output = Pred<Pred<<(T, U) as TypeSub>::Output>>;
}
