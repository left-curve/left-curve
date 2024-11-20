import type { Address } from "../address.js";

export type DangoAppConfigResponse = {
  addresses: {
    accountFactory: Address;
    ibcTransfer: Address;
    tokenFactory: Address;
    lending: Address;
    oracle: Address;
    amm: Address;
  };
};
