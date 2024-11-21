# Margin account: health

The [dango-lending](https://github.com/left-curve/left-curve/tree/main/dango/lending) contract stores a **collateral power** for each collateral asset, and a `Market` for each borrowable asset:

```rust
const COLLATERAL_POWERS: Item<BTreeMap<Denom, Udec128>> = Item::new("collateral_power");

const MARKETS: Map<&Denom, Market> = Map::new("market");
```

- An asset may be a collateral asset but not a borrowable asset, e.g. wstETH, stATOM, LP tokens. But typically all borrowable assets are also collateral assets, such that when a margin account borrows an asset, this asset counts both as collateral and debt.
- Collateral powers are to be bounded in the range `[0, 1)`. An asset with lower volatility and more abundant liquidity gets bigger collateral power, vise versa.
- We may store all collateral powers in a single `Item<BTreeMap<Denom, Udec128>>` if we don't expect to support too many collateral assets.

Suppose:

- a margin account has collateral assets $A_1, A_2, \dots, A_n$ and debts $B_1, B_2, \dots, B_m$
- the price of an asset $X$ is $P_X$
- the collateral power of an asset $X$ is $C_X$

The account's **utilization** is:

$$
U = \frac{ \sum_{j=1}^m P_{B_j} B_j }{ \sum_{i=1}^n C_{A_i} P_{A_i} A_i }
$$

In the `backrun` function, the margin account asserts $U \leq 1$. If not true, it throws an error to revert the transaction.

The frontend should additionally have a `max_ltv`, somewhat smaller than 1, such as 95%. It should warn or prevent users from doing anything that results in their utilization going bigger than this, such that their account isn't instantly liquidated.
