# Liquidation

This is a short note to explain the math behind margin account liquidation. Dango's margin account liquidation is based on Euler's dynamic liquidation bonus method, or "Dutch Liquidation". Please see this [paper](https://docs.euler.finance/Dutch_Liquidation_Analysis.pdf) for more details.

## Defintions

- **Collateral power**: The adjustment factor for the collateral value of a given collateral token. Meaning, if the collateral token has a collateral power of 0.9, then the value of the collateral token is 90% of its actual value.
- **Utilization**: The ratio of the total value of the debts to the total value of the account's collateral adjusted for the collateral power. When the utilization rate is greater than 1, the account is undercollateralized and can be liquidated.
- **Health factor**: The inverse of the utilization rate. When the health factor is less than 1, the account is undercollateralized and can be liquidated.
- **Liquidation bonus**: The amount of extra collateral that is awarded to the liquidator to incentivize them to liquidate the account.

## Calculating the maximum repayable debt

When liquidating the account, we set a **target utilization rate** (which is a fraction between 0 and 1) that we want to reach after the liquidation. We then calculate the **maximum amount of debt that can be repaid** to reach this target utilization rate.

In our system, the liquidation bonus is defined as a function of the health factor:

$$
\mathtt{liquidationBonus} = 1 - \mathtt{healthFactor}
$$

This means that the more undercollateralized the account is, the higher the liquidation bonus.

Consider the below graph showing an undercollateralized margin account:

![undercollateralized margin account](./liquidation-graph.png)

The x-axis represents the value of account's debt and the y-axis represents the value of account's collateral _adjusted for the collateral power_. The point $(d_0, c_0)$ is the account's current status. Since it is below the orange line, it is undercollateralized and eligible for liquidation. The blue line represents the **trajectory of the liquidation**. The black line represents all points at which the account's utilization rate is equal to the target utilization rate. The maximum repayable debt therefore is found at the intersection of the blue line and the black line.

### Liquidation trajectory

We can think of a user’s position as a point $(d, c)$ in the debt and adjusted collateral plane. To derive the equation for the **liquidation trajectory**, we can use the point slope form of the equation for a line:

$$
c - c_0 = m(d - d_0)
$$

where $m$ is the slope or gradient of the line, and $(d_0, c_0)$ is a known point on the line (in this case, it is the position's current status).

For:

- $\Delta d$, value of debt to be paid;
- $b_0 = 1 - c_0 / d_0$, the liquidation bonus corresponding to the position's current status;

the gradient is given by

$$
m = \frac{c - c_0}{d - d_0}
  = \frac{(c_0 - (1 + b_0)\Delta d) - c_0}{(d_0 − \Delta d) − d_0}
  = 1 + b_0
$$

Therefore, we can rewrite the equation for the liquidation trajectory as:

$$
c - c_0 = m(d - d_0) = (1 + b_0)(d - d_0)
$$

$$
c = c_0 + (1 + b_0)(d - d_0)
$$

### Target health factor

Since the health factor is the value of the collateral _adjusted for the collateral power_ divided by the value of the debts, we can express it as:

$$
\mathtt{healthFactor} = \frac{c}{d}
$$

Therefore, we can rewrite the equation for the line which defines all points at which the health factor is equal to the **target health factor** $H$ as:

$$
c = H \cdot d
$$

### Maximum repayable debt

The maximum repayable debt is found at the intersection of the liquidation trajectory and the line which defines all points at which the health factor is equal to the target health factor. To find this intersection, we set the two above equations equal to each other:

$$
H \cdot d = c_0 + (1 + b_0)(d - d_0)
$$

Solving for $d$:

$$
d = \frac{c_0 - (1 + b_0)d_0}{H - (1 + b_0)}
$$

This $d$ defines the amount of debt after the liquidation if the new health factor is equal to the target health factor. Thus, the maximum repayable debt is equal to the prior debt minus this amount:

$$
\mathtt{maxRepayableDebt} = d_0 - d = d_0 - \frac{c_0 - (1 + b_0)d_0}{H - (1 + b_0)}
$$
