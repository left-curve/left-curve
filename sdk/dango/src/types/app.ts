import type { Address } from "@left-curve/sdk/types";

export type AppConfig = {
  addresses: {
    accountFactory: Address;
    ibcTransfer: Address;
    tokenFactory: Address;
    lending: Address;
    oracle: Address;
    dex: Address;
    warp: Address;
    hyperlane: {
      fee: Address;
      ism: Address;
      mailbox: Address;
      merkle: Address;
      va: Address;
    };
  };
};
