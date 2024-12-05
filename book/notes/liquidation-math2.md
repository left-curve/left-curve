Let the following variables be defined as:

- `d`: Value of the debt;
- `c`: Value of the collateral adjusted by the collaterals powers;
- `p`: Power of the collateral to liquidate (e.g.: `0.8`);
- `t`: Target of the final ratio $c/d$;
- `b`: Liquidation bonus (ex: `0.1`);

Liquidation is possible if and only if:

$$\frac{c}{d} < 1$$

Let $c_1$ and $d_1$ represent the `collateral` and `debt` values, respectively, ***after liquidation***. The target ratio $t$ is defined as:

$$
t = \frac{c_1}{d_1}
$$

Let $x$ be the amount of debt to liquidate in order to reach the target $t$. After liquidation, the updated values are:
$$c_1 = c - x * p * (1+b)$$
$$d_1 = d - x$$

Substituting these into the definition of $t$, we obtain:

$$
t = \frac{c - x * p * (1 + b)}{d - x}
$$

Solving for $x$:

$$
x = \frac{t * d - c}{t - p * (1+b)}
$$

The solution is valid under the following conditions:

- $t>1$ (because liquidation can happens only if $d>c$)
- $\frac{c_!}{d_1} > \frac{c}{d}$: Otherwise the ratio after the liquidation is lower compared ratio before liquidation. $c, d, p$ are fixed, so this create the following condition: $ b < \frac{c}{d*p} - 1 $
