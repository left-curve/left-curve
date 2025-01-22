import type { Address } from "@left-curve/types";

export type AppConfig = {
  addresses: {
    accountFactory: Address;
    ibcTransfer: Address;
    tokenFactory: Address;
    lending: Address;
    oracle: Address;
    amm: Address;
  };
};
