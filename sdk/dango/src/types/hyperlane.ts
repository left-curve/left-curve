import type { Address } from "@left-curve/sdk/types";

export type Addr32 = `0x${string}`;
export type MailBoxConfig = {
  localDomain: Domain;
  defaultIsm: Address;
};

export type Domain = number;

export type WarpRemote = {
  domain: Domain;
  contract: Addr32;
};

export type BitcoinRemote = "bitcoin";

export type Remote = { warp: WarpRemote } | BitcoinRemote;
