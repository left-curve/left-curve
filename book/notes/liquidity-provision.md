# Liquidity provision

Given a liquidity pool consisting of two assets, A and B, and the invariant $I(A, B)$, where $A$ and $B$ are the amounts of the two assets in the pool (the "**pool reserve**"). For simplicity, we denote this as $I$.

Suppose a user provides liquidity with amounts $a$ and $b$. After the liquidity is added, the invariant value is $I(A + a, B + b)$. For simplicity, we denote this as $I'$.

Suppose before adding the liquidity, the supply of LP token is $L$. We mint user new LP tokens of the following amount:

$$
l = (1 - f) \left( \frac{I'}{I} - 1 \right) L
$$

Here, $f$ is a fee rate we charge on the amount of LP tokens mint. Without this fee, the following exploit would be possible: provide unbalanced liquidity, then immediately withdraw balanced liquidity. This effectively achieves a fee-less swap.

The fee rate should be a function over $(A, B, a, b)$, **reflecting how unbalance the user liquidity is**:

- If user liquidity is prefectly balanced, that is, $\frac{a}{A} = \frac{b}{B}$, fee rate should be zero: $f(A, B, a ,b) = 0$.
- If user liquidity is prefertly unbalanced, that is, **one-sided** (e.g. $a \ne 0$ but $b = 0$), then the fee rate should be a value such that if the attack is carried out, the output is equal to doing a swap normally.

Our objective for the rest of this article, is to **work out the expression of the fee function** $f(A, B, a, b)$.

## Fee rate

Consider the case where the user liquidity is unbalanced. Without losing generality, let's suppose $\frac{a}{A} > \frac{b}{B}$. That is, the user provides a more than abundant amount of A, and a less than sufficient amount of B.

### Scenario 1

In the first scenario, the user withdraws liquidity immediately after provision. He would get:

$$
\frac{l}{L + l} (A + a)
$$

$$
\frac{l}{L + l} (B + b)
$$

Here, $\frac{l}{L + l}$ is the portion of the pool's liquidity owned by the user. We can work out its expression as:

$$
\frac{l}{L + l} = \frac{\frac{I'}{I} - 1}{\frac{I'}{I} - f} = \frac{r - 1}{r - f}
$$

where $r = \frac{I'}{I}$, which represents how much the invariant increases as a result of the added liquidity.

### Scenario 2

In the second scenario, the user does a swap of $\Delta a$ amount of A into $(1 - s) \Delta b$ amount of B, where $s$ is the swap fee rate, which is a constant. The swap must satisfy the invariant:

$$
I(A, B) = I(A + \Delta a, B - \Delta b)
$$

The user now has $A - \Delta a$ amount of A and $B + (1 - s) \Delta b$ amount of B.

As discussed in the previous section, we must choose a fee rate $f$ **such that the two scenarios are equivalent**. This means the user ends up with the same amount of A and B in both scenarios:

$$
\frac{r - 1}{r - f} (A + a) = A - \Delta a
$$

$$
\frac{r - 1}{r - f} (B + b) = B + (1 - s) \Delta b
$$

We can rearrange these into a cleaner form:

$$
\frac{A + a}{A - \Delta a} = \frac{B + b}{B + (1 - s) \Delta b}
$$

$$
f = r - (r - 1) \frac{A + a}{A - \Delta a} = r - (r - 1) \frac{B + b}{B + (1 - s) \Delta b}
$$

We can use the first equation to work out either $\Delta a$ or $\Delta b$, and put it into the second equation to get $f$.

## Xyk pool

The xyk pool has the invariant:

$$
I(A, B) = A B
$$

Our previous system of equations takes the form:

$$
A B = (A + \Delta a) (B - \Delta b)
$$

$$
\frac{A + a}{A - \Delta a} = \frac{B + b}{B + (1 - s) \Delta b}
$$

$$
f = r - (r - 1) \frac{A + a}{A - \Delta a}
$$

> TODO...
