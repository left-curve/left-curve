# Orderbook Math

This document explains the mathematical concepts behind calculating the maximum swap input that respects a limit order price for pools.

## Definition

- $X$: amount of Token1 in pool
- $Y$: amount of Token2 in pool
- $i$: input swap (referred to Token1)
- $o$: output swap (referred to Token2)
- $P_s$: price of a specific swap
- $P_w$: price desired for a limit order
- $P_a$: price desired for a limit order, adjusted based on whether the order is a buy or sell

## Concept

Each limit order specifies a price and amount at which the user wants to buy or sell the base asset, either at the specified price or a better one.
Each pool contains two tokens, the first one is the `base_asset` and the second is the `quote_asset`.

- The `base_asset` is the one you want to buy or sell.
- The `quote_asset` is used to value the `base_asset`.

  For example, for the pool BTC/USDC, BTC is the `base_asset` and USDC is the `quote_asset`.

For both buy and sell orders, the price will always be expressed in terms of `quote_asset`.

The functions that ensure a swap respects a specific $P_w$ for a limit order differs a little bit depending if the order is a **sell** or a **buy**.

- If we want to **sell** 1 `BTC` for `USDC` at $P_w$ of 100k, all swaps with a $P_s$ equal or higher than 100k are valid.

    To ensure that $P_w$ is respected, we must verify that $\frac{USDC\ amount}{BTC\ amount} = \frac{output}{input} \geq P_w$, where `BTC` is the **input** and `USDC` the **output**.

- If we want to **buy** 1 `BTC` with `USDC` at $P_w$ of 100k, all swaps with a $P_s$ equal or lower than 100k are valid.

    To ensure that $P_w$ is respected, we must verify that $\frac{USDC\ amount}{BTC\ amount} = \frac{input}{output} \leq P_w$, where `USDC` is the **input** and `BTC` the **output**.

Since both conditions involve comparing a ratio of output to input, we can unify the price check by defining the swap price $P_s$ as:
$$
P_s = \frac{output}{input}
$$

Using this definition, we can rewrite the price-checking conditions for both buy and sell orders:

- **Sell order:**
  The inequality for a sell order already matches the $P_s$ format, since we have `USDC` as output and `BTC` as input and so $\frac{USDC\ amount}{BTC\ amount} = \frac{output}{input} = P_s$.

- **Buy order:**
  Here, `USDC` is the input and `BTC` is the output, meaning the ratio $\frac{USDC\ amount}{BTC\ amount} = \frac{input}{output}$ is the inverse of $P_s$.
  To transform the inequality into the $P_s$ format, we raise both sides to the power of $-1$. Since both sides are positive, the inequality sign flips:

  $$
  \frac{input}{output} \leq P_w
  $$
  $$
  (\frac{input}{output})^{-1} \geq P_w^{-1}
  $$
  $$
  \frac{output}{input} \geq \frac{1}{P_w}
  $$
  $$
  P_s \geq \frac{1}{P_w}
  $$

By defining the adjusted price $P_a$ based on the type of order:

- **Sell order:** $P_a = P_w$
- **Buy order:** $P_a = \frac{1}{P_w}$

We can unify the price-check function into a single condition for both buy and sell orders:
$$
P_s \geq P_a
$$

## XYK Pool

The constant product formula for the `XYK` pool is: $YX = (Y - o) * (X + i)$.

To calculate the output, the formula became:

$$
o = \frac{Y*i}{X + i}
$$

Given a limit order with a certain price $P_a$ we have to ensure that
$$
P_s \geq P_a
$$
$$
\frac{output}{input} \geq P_a
$$
$$
\frac{o}{i} \geq P_a
$$

To find the maximum input we need to find $i$ for the equation $P_a = P_s$
$$
P_a = \frac{o}{i} = \frac{\frac{Yi}{X + i}}{i}
$$

Solving the equation for $i$:
$$
P_a = \frac{\frac{Yi}{X + i}}{i}\
$$

$$
P_ai = \frac{Yi}{X + i}
$$

$$
P_ai * (X + i) = Yi
$$

$$
P_ai^2 + P_aXi - Yi = 0
$$

$$
P_ai^2 + i(P_aX - Y) = 0
$$

$$
i^2 + i(X - \frac{Y}{P_a}) = 0
$$

$$
i( i + (X - \frac{Y}{P_a})) = 0
$$

Since $i=0$ represents no swap, it is not relevant. We consider only the second solution:

$$
i + X - \frac{Y}{P_a} = 0
$$

$$
i = \frac{Y}{P_a} - X
$$

This solution is valid only if $\frac{Y}{P_a} - X > 0$, so when $\frac{Y}{P_a} > X$, otherwise the input amount would be negative.
