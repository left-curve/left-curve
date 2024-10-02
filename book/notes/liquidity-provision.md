# Liquidity provision

Objective: allow users to provided _unbalanced_ or _one sided_ liquidity.

## Xyk pools

Consider the pool of assets, A and B.

Assume the pool contains $A_{\mathrm{pool}}$ units of A and $B_{\mathrm{pool}}$ units of B.

A user wishes to provide $A_{\mathrm{user}}$ units of A and $B_{\mathrm{user}}$ units of B to the pool.

The user's liquidity is said to be _balanced_ if $\frac{A_{\mathrm{user}}}{A_{\mathrm{pool}}} = \frac{B_{\mathrm{user}}}{B_{\mathrm{pool}}}$. However, this is rarely the case.

Let's consider the case where $\frac{A_{\mathrm{user}}}{A_{\mathrm{pool}}} > \frac{B_{\mathrm{user}}}{B_{\mathrm{pool}}}$. Here, asset A is more than abundant, while B is less than sufficient.

To make it balanced, we're going to swap a portion of user's A to B. Let's say that we swap $A_{\mathrm{in}}$ for $B_{\mathrm{out}}$. The constant product variant holds during the swap:

```math
\begin{equation}
A_{\mathrm{pool}} B_{\mathrm{pool}} = (A_{\mathrm{pool}} + A_{\mathrm{in}}) (B_{\mathrm{pool}} - B_{\mathrm{out}})
\end{equation}
```

(Note: we don't charge any fee for this swap.)

Our objective is that after the swap, the user's liquidity becomes _balanced_. That is,

```math
\begin{equation}
\frac{A_{\mathrm{user}} - A_{\mathrm{in}}}{A_{\mathrm{pool}} + A_{\mathrm{in}}} = \frac{B_{\mathrm{user}} + B_{\mathrm{out}}}{B_{\mathrm{pool}} - B_{\mathrm{out}}}
\end{equation}
```

Combining equations (1) and (2) we can solve that:

```math
\begin{align*}
A_{\mathrm{in}} &= \sqrt{\frac{A_{\mathrm{pool}} + A_{\mathrm{user}}}{B_{\mathrm{pool}} + B_{\mathrm{user}}} A_{\mathrm{pool}} B_{\mathrm{pool}}} - A_{\mathrm{pool}} \\
B_{\mathrm{out}} &= \sqrt{\frac{B_{\mathrm{pool}} + B_{\mathrm{user}}}{A_{\mathrm{pool}} + A_{\mathrm{user}}} A_{\mathrm{pool}} B_{\mathrm{pool}}} - B_{\mathrm{pool}}
\end{align*}
```

This is however now very useful - no matter how many A we swap for B, after the swap and liquidity provision, the pool will always have $(A_{\mathrm{pool}} + A_{\mathrm{user}})$ of A and $(B_{\mathrm{pool}} + B_{\mathrm{user}})$ of B in liquidity. What we care about it how many liquidity share tokens to be minted to the user.

The user's share in the pool after the swap is defined by the ratio $\frac{A_{\mathrm{user}} - A_{\mathrm{in}}}{A_{\mathrm{pool}} + A_{\mathrm{in}}}$, which can be solved as:

```math
\mathrm{share\%} = \sqrt{\frac{A_{\mathrm{pool}} + A_{\mathrm{user}}}{A_{\mathrm{pool}}}} \sqrt{\frac{B_{\mathrm{pool}} + B_{\mathrm{user}}}{B_{\mathrm{pool}}}} - 1
```

Let's do some basic sanity checks of this formula.

- Suppose the user provides zero liquidity, i.e. $A_{\mathrm{user}} = B_{\mathrm{user}} = 0$. In this case,

```math
\begin{align*}
\mathrm{share\%} &= \sqrt{\frac{A_{\mathrm{pool}} + 0}{A_{\mathrm{pool}}}} \sqrt{\frac{B_{\mathrm{pool}} + 0}{B_{\mathrm{pool}}}} - 1 \\
&= \sqrt{1 \times 1} - 1 \\
&= 0
\end{align*}
```

- Suppose the user provides balanced liquidity, i.e. $A_{\mathrm{user}} = \epsilon A_{\mathrm{pool}}$ and $B_{\mathrm{user}} = \epsilon B_{\mathrm{pool}}$, where $\epsilon > 0$ is a constant. In this case,

```math
\begin{align*}
\mathrm{share\%} &= \sqrt{\frac{(1 + \epsilon) A_{\mathrm{pool}}}{A_{\mathrm{pool}}}} \sqrt{\frac{(1 + \epsilon) B_{\mathrm{pool}}}{B_{\mathrm{pool}}}} - 1 \\
&= \sqrt{(1 + \epsilon)^2} - 1 \\
&= \epsilon
\end{align*}
```
