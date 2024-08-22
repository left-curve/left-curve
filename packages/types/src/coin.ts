export type Coin = {
  denom: string;
  amount: string;
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
export type Coins = Record<string, string>;
