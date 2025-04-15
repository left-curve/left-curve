import type { Denom, Prettify, Price } from "@left-curve/dango/types";

export type CoinGeckoId = string;

export type WithPrice<T> = T & {
  readonly price: Price;
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

export type AlloyCoin = Prettify<
  BaseCoin & {
    readonly type: "alloyed";
  }
>;

export type CoinFee = WithGasPriceStep<NativeCoin>;

export type AnyCoin = NativeCoin | AlloyCoin | ContractCoin;
