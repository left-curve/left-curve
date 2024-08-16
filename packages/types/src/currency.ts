export type WithGasPriceStep<T> = T & {
  readonly gasPriceStep: {
    readonly low: number;
    readonly average: number;
    readonly high: number;
  };
};

export interface BaseCurrency {
  readonly symbol: string;
  readonly name: string;
  readonly denom: string;
  readonly decimals: number;
}

export interface NativeCurrency extends BaseCurrency {
  readonly type: "native";
}

export interface CW20Currency extends BaseCurrency {
  readonly type: "cw-20";
  readonly contractAddress: string;
}

export interface IBCCurrency extends BaseCurrency {
  readonly type: "ibc";
  readonly portId: string;
  readonly channelId: string;
  readonly origin: {
    readonly portId: string;
    readonly channelId: string;
    readonly asset: Currency;
  };
}

export type FeeCurrency = WithGasPriceStep<NativeCurrency | IBCCurrency>;

export type Currency = NativeCurrency | CW20Currency | IBCCurrency;
