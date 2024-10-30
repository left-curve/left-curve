import type { Prettify } from "./utils.js";

export type CoinGeckoId = string;
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

export type WithGasPriceStep<T> = T & {
  readonly gasPriceStep: {
    readonly low: number;
    readonly average: number;
    readonly high: number;
  };
};

export type BaseCoin = {
  readonly symbol: string;
  readonly name: string;
  readonly denom: Denom;
  readonly decimals: number;
  readonly logoURI?: string;
  readonly coingeckoId?: CoinGeckoId;
};

export type NativeCoin = Prettify<
  BaseCoin & {
    readonly type: "native";
  }
>;

export type ContractCoin = Prettify<
  BaseCoin & {
    readonly type: "contract";
    readonly contractAddress: string;
  }
>;

export type IBCCoin = Prettify<
  BaseCoin & {
    readonly type: "ibc";
    readonly portId: string;
    readonly channelId: string;
    readonly origin: {
      readonly portId: string;
      readonly channelId: string;
      readonly asset: AnyCoin;
    };
  }
>;

export type CoinFee = WithGasPriceStep<NativeCoin>;

export type AnyCoin = NativeCoin | IBCCoin | ContractCoin;
