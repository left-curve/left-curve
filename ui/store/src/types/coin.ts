import type { Denom, Prettify } from "@left-curve/dango/types";

export type WithPrice<T> = T & {
  readonly price: string;
};

export type WithAmount<T> = T & {
  readonly amount: string;
};

export type WithBalance<T> = T & {
  readonly balance: string;
};

export type WithGasPriceStep<T> = T & {
  readonly gasPriceStep: {
    readonly low: number;
    readonly average: number;
    readonly high: number;
  };
};

export type BaseCoin = {
  readonly symbol: string;
  readonly denom: Denom;
  readonly decimals: number;
  readonly name: string;
  readonly logoURI?: string;
};

export type NativeCoin = Prettify<
  BaseCoin & {
    readonly type: "native";
  }
>;

export type LpCoin = Prettify<
  BaseCoin & {
    readonly type: "lp";
    readonly base: AnyCoin;
    readonly quote: AnyCoin;
  }
>;

export type CoinFee = WithGasPriceStep<NativeCoin>;

export type AnyCoin = NativeCoin | LpCoin;
