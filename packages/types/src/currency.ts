import type { Prettify } from "./utils";

export type WithGasPriceStep<T> = T & {
  readonly gasPriceStep: {
    readonly low: number;
    readonly average: number;
    readonly high: number;
  };
};

export type BaseCurrency = {
  readonly symbol: string;
  readonly name: string;
  readonly denom: string;
  readonly decimals: number;
};

export type NativeCurrency = Prettify<
  BaseCurrency & {
    readonly type: "native";
  }
>;

export type CW20Currency = Prettify<
  BaseCurrency & {
    readonly type: "cw-20";
    readonly contractAddress: string;
  }
>;

export type IBCCurrency = Prettify<
  BaseCurrency & {
    readonly type: "ibc";
    readonly portId: string;
    readonly channelId: string;
    readonly origin: {
      readonly portId: string;
      readonly channelId: string;
      readonly asset: Currency;
    };
  }
>;

export type FeeCurrency = WithGasPriceStep<NativeCurrency | IBCCurrency>;

export type Currency = NativeCurrency | CW20Currency | IBCCurrency;
