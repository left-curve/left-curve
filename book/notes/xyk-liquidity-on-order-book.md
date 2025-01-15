# Xyk liquidity on order book

Dango provides passive liquidity pools for its order book DEX. For a major pair such as BTC-USDC, the pool places orders following the classic [Avellaneda-Stoikov strategy](https://people.orie.cornell.edu/sfs33/LimitOrderBook.pdf). However, this strategy requires an oracle for the asset's price, which isn't available for many long tail assets such as newly launched memecoins. For such trading pairs, we propose a passive liquidity pool type following the $x \cdot y = k$ invariant.

Consider a trading pair consisting of base asset $\mathtt{A}$ and quote asset $\mathtt{B}$. A passive liquidity pool containing $A$ units of asset $\mathtt{A}$ and $B$ units of asset $\mathtt{B}$ places orders on the order book **following the XYK curve**. What will these orders be like?

Let's think of this this way: consider we swap $a$ units of asset $\mathtt{A}$ for asset $\mathtt{B}$. The output amount, $b$, can be found by:

$$
A \cdot B = (A + a) (B - b)
$$

The price of this trade, $p$, defined as the amount of quote asset $\mathtt{B}$ per unit of base asset $\mathtt{A}$, is:

$$
p = \frac{b}{a}
$$

We can interpret the result here as follows: **the pool is willing to BUY $a$ units of asset $\mathtt{A}$ at price $p$ or better**.

Let's consider another trade at a different price $p'$:

$$
A \cdot B = (A + a') (B - b')
$$

$$
p' = \frac{b'}{a'}
$$

With some arithmetic manipulations, we can find that:

$$
\Delta a = a' - a = \frac{p - p'}{p p'} B
$$

This means **between the prices $p$ and $p'$, the pool would place orders with a total size of $\Delta a$**.

Let's use an example to understand what these mean.

## Example

Suppose the pool contains reserves of $A$ = 2 BTC and $B$ = 200,000 USDC. The pool places orders at a **tick size** of $\Delta p$ = 1 USDC.

These reserves indicates a price of $\frac{B}{A}$ = 100,000 USDC per BTC. Naturally, the pool will place BUY orders at prices under 100,000, and SELL orders above 100,000; it will not place any order at exacly 100,000. Let's look only at the BUY side for now.

Let's set $p$ = 100,000 and $p' = p - \Delta p$ = 99,999. We can solve:

$$
\Delta a = \frac{100,000 - 99,999}{100,000 \times 99,999} \times 200,000 = 0.0000200002
$$

In order words, the pool will place a BUY order at 99,999 USDC per BTC with a size of 0.0000200002 BTC. Because BTC has only 8 decimal places, this rounds down to 0.00002.

Now, set $p$ = 99,999 and $p' = p - \Delta p$ = 99,998:

$$
\Delta a = \frac{99,999 - 99,998}{99,999 \times 99,998} \times 200,000 = 0.0000200006
$$

This means the pool will place an order at 99,998 USDC per BTC with, again, a size of 0.00002 BTC. However, if not for rounding, the size will be slightly bigger than the previous tick.

Repeating this process, we can find the order sizes at all ticks (note: this computation can be parallelized):

| Price (USDC per BTC) | Order Size (BTC) |
| -------------------- | ---------------- |
| 99,999               | 0.00002000       |
| ...                  | ...              |
| 99,998               | 0.00002000       |
| ...                  | ...              |
| 95,000               | 0.00002216       |
| ...                  | ...              |
| 90,000               | 0.00002469       |
| ...                  | ...              |
| 85,000               | 0.00002768       |
| ...                  | ...              |
| 80,000               | 0.00003124       |
