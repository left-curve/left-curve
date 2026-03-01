# Deposit & Withdrawal

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

Internal logics of the perps contract use USD amounts exclusively.

## 2 Trader Deposit

The user sends settlement currency as attached funds. The perps contract:

1. Queries the oracle for the settlement-currency price.
2. Converts the token amount to USD: $\mathtt{depositValue} = \mathtt{amount} \times \mathtt{price}$.
3. Credits `userState.margin` by $\mathtt{depositValue}$.

The tokens remain in the perps contract's bank balance.

## 3 Trader Withdraw

The user specifies how much USD margin to withdraw. The perps contract:

1. Computes available margin (equity minus used margin minus reserved margin),
   clamped to zero.
2. Ensures the requested amount does not exceed available margin.
3. Deducts the amount from `userState.margin`.
4. Converts USD to settlement-currency tokens at the current oracle price
   (floor-rounded for safety — the contract keeps slightly more than strictly
   needed).
5. Transfers the tokens to the user.
