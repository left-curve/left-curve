import type { Address } from "@left-curve/types";

export type AppConfigResponse = {
  addresses: {
    accountFactory: Address;
    ibcTransfer: Address;
    tokenFactory: Address;
    lending: Address;
    oracle: Address;
    amm: Address;
  };
};
