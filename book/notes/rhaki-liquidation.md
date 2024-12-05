Let the following variables be defined as:

- `d`: Value of the debt;
- `c`: Value of the collateral adjusted by the collaterals powers;
- `p`: Power of the collateral to liquidate (e.g.: `0.8`);
- `t`: Target of the final ratio $c/d$;
- `b`: Liquidation bonus (ex: `0.1`);

Liquidation is possible if and only if:

$$\frac{c}{d} < 1$$

Let $c_1$ and $d_1$ represent the `collateral` and `debt` values, respectively, after liquidation. The target ratio $t$ is defined as:

$$
t = \frac{c_1}{d_1}
$$

Let $x$ be the amount of debt to liquidate in order to reach the target $t$. After liquidation, the updated values are:
$$d_1 = d - x$$
$$c_1 = c - x * p * (1+b)$$

Substituting these into the definition of $t$, we obtain:

$$
t = \frac{d - x}{ c - x * p * (1+b)}
$$

Solving for x:

$$
x = \frac{t*d - c}{t - p * (1+b)}
$$

The solution is valid under the following conditions:

- $t>1$ (because liquidation can happens only if $c>d$)
- $t>p*(1+b)$: This must to be ensure when a new collateral is registered or when `max_liqudation_bonus` / `t` is updated

Anyway there are other conditions:

- Liquidation bonus ($b$) must be lesser then
  $\frac{c}{d * p}$
  otherwise the bonus is to high and not increase the ratio after liquidation.
- $p*(1+b)<1$. This must to be ensure when a new collateral is registered or when `max_liqudation_bonus` is updated
