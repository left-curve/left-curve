## Bounded values

A common situation developers find themselves in is their contract needs to take a value that must been within a certain bound.

For example, a fee rate should be within the range of 0~1. It doesn't make sense to charge more than 100% fee. Whenever a fee rate is provided, the contract needs to verify it's within the bounds, throwing error if not:

```rust
#[grug::derive(Serde)]
struct InstantiateMsg {
    pub fee_rate: Udec256,
}

#[grug::export]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    ensure!(
        Udec256::ZERO <= fee_rate && fee_rate < Udec256::ONE,
        "fee rate is out of bounds"
    );

    Ok(Response::new())
}
```

We call this an **imperative** approach for working with bounded values.

The problem with this is that the declaration and validation of `fee_rate` are in two places, often in two separate files. Sometimes developers simply forget to do the validation.

Instead, Grug encourages a **declarative** approach. We declare the valid range of a value at the time we define it, utilizing the `Bounded` type and `Bounds` trait:

```rust
use grug::{Bounded, Bounds};
use std::ops::Bound;

struct FeeRateBounds;

impl Bounds<Udec256> for FeeRateBounds {
    const MIN: Bound<Udec256> = Bound::Inclusive(Udec256::ZERO);
    const MAX: Bound<Udec256> = Bound::Exclusive(Udec256::ONE);
}

type FeeRate = Bounded<Udec256, FeeRateBounds>;

#[grug::derive(Serde)]
struct InstantiateMsg {
    pub fee_rate: FeeRate,
}

#[grug::export]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // No need to validate the fee rate here.
    // Its bounds are already verified when `msg` is deserialized!

    Ok(Response::new())
}
```

This seems a bit verbose, so a `declare_bounded` macro is provided to simplify it:

```rust
use grug::declare_bounded;

declare_bounded! {
    name = FeeRate,
    type = Udec256,
    min = Bound::Inclusive(Udec256::ZERO),
    max = Bound::Exclusive(Udec256::ONE),
}

#[grug::derive(Serde)]
struct InstantiateMsg {
    pub fee_rate: FeeRate,
}

#[grug::export]
fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    Ok(Response::new())
}
```
