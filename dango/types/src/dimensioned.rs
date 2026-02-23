//! A Rust type system that encapsulates the dimensionality of values.

use {
    grug::{Dec128_6, MathResult, Number},
    std::marker::PhantomData,
};

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

/// A value with three dimensions: quantity, USD, and time duration.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value<Q, U, D> {
    inner: Dec128_6,
    _quantity: PhantomData<Q>,
    _usd: PhantomData<U>,
    _duration: PhantomData<D>,
}

impl<Q, U, D> Value<Q, U, D> {
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
        rhs: Value<Q1, U1, D1>,
    ) -> MathResult<
        Value<
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
        self.inner.checked_mul(rhs.inner).map(Value::new)
    }
}
