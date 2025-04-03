export type Denom = string;

export type Coin = {
  readonly denom: Denom;
  readonly amount: string;
};

/**
 * Coins is a record where the coin's denomination is used as the key
 * and the amount is used as the value.
 * @example
 * ```typescript
 * {
 * uusdc: "1000000",
 * uosmo: "1000000",
 * }
 * ```
 */
export type Coins = Record<Denom, string>;

export type Funds = Record<Denom, string>;
