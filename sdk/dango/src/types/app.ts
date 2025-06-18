import type { Address } from "@left-curve/sdk/types";

export type AppConfig = {
  addresses: {
    accountFactory: Address;
    gateway: Address;
    lending: Address;
    oracle: Address;
    dex: Address;
    warp: Address;
    taxman: Address;
    hyperlane: {
      ism: Address;
      mailbox: Address;
      va: Address;
    };
  };
  makerFeeRate: string;
  takerFeeRate: string;
  maxLiquiditationBonus: string;
  minLiquiditationBonus: string;
  targetUtilizationRate: string;
};
