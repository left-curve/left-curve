# Settlement

## 1 Overview

All trader margin is held **internally** in the perps contract as a USD value
(`UsdValue`) on each user's state. PnL and fee settlement is pure USD
arithmetic — no token conversions are needed during matching or liquidation.

Token conversion only happens at two boundaries:

- **Deposit** — the user sends settlement currency (USDC) to the perps
  contract; the oracle price converts the token amount to USD and credits
  `userState.margin`.
- **Withdraw** — the user requests a USD amount; the oracle price converts it
  to settlement-currency tokens (floor-rounded) and transfers them out.

This design eliminates rounding errors that accumulate from repeated USD-to-token
conversions, reduces cross-contract calls, and keeps margining logic entirely
inside the perps contract.

## 2 Trader Deposit

```
ExecuteMsg::Deposit {}
```

The user sends settlement currency as attached funds. The perps contract:

1. Queries the oracle for the settlement-currency price.
2. Converts the token amount to USD: $\mathtt{depositValue} = \mathtt{amount} \times \mathtt{price}$.
3. Credits `userState.margin` by $\mathtt{depositValue}$.

The tokens remain in the perps contract's bank balance.

## 3 Trader Withdraw

```
ExecuteMsg::Withdraw { margin: UsdValue }
```

The user specifies how much USD margin to withdraw. The perps contract:

1. Computes available margin (equity minus used margin minus reserved margin),
   clamped to zero.
2. Ensures the requested amount does not exceed available margin.
3. Deducts the amount from `userState.margin`.
4. Converts USD to settlement-currency tokens at the current oracle price
   (floor-rounded for safety — the contract keeps slightly more than strictly
   needed).
5. Transfers the tokens to the user.

## 4 PnL settlement (`settle_pnls`)

During order matching and liquidation, the contract accumulates per-user PnL
and fee maps (`BTreeMap<Addr, UsdValue>`). After all fills are computed, a
single `settle_pnls` call applies everything in place:

### Fee loop (runs first)

For each non-vault user with a fee:

$$
\mathtt{userState.margin} \mathrel{-}= \mathtt{fee}
$$

$$
\mathtt{state.vaultMargin} \mathrel{+}= \mathtt{fee}
$$

Fees from the vault to itself are skipped (no-op).

### PnL loop

**Non-vault users:**

$$
\mathtt{userState.margin} \mathrel{+}= \mathtt{pnl}
$$

A user's margin can go negative temporarily — the outer function handles bad
debt (see [Liquidation](liquidation-and-adl.md)).

**Vault:**

Profit is applied to `state.vaultMargin`, first repaying any existing
`vaultDeficit`:

$$
\mathtt{repaid} = \min(\mathtt{pnl},\; \mathtt{vaultDeficit})
$$

$$
\mathtt{vaultDeficit} \mathrel{-}= \mathtt{repaid}
$$

$$
\mathtt{vaultMargin} \mathrel{+}= \mathtt{pnl} - \mathtt{repaid}
$$

Loss absorbs from `vaultMargin`; any shortfall becomes `vaultDeficit`:

$$
\mathtt{absorbed} = \min(|\mathtt{pnl}|,\; \mathtt{vaultMargin})
$$

$$
\mathtt{vaultMargin} \mathrel{-}= \mathtt{absorbed}
$$

$$
\mathtt{vaultDeficit} \mathrel{+}= |\mathtt{pnl}| - \mathtt{absorbed}
$$

### Why no payouts or collections

Under the old design, settlement produced token transfers (payouts to winners,
collections from losers). With internalized margin, all changes are in-memory
mutations to `userState.margin` and `state.vaultMargin`. No messages are
emitted from `settle_pnls`.

## 5 Vault margin

The vault's margin (`state.vaultMargin`) is a `UsdValue` that tracks:

- LP deposits (via `AddLiquidity` → converted at oracle price).
- Trading fees earned from all non-vault users.
- The vault's own realized PnL from market-making and backstop fills.

It is **not** a bank balance — it is an internal accounting value. Actual
tokens only move when LPs claim unlocks or traders deposit/withdraw.
