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

export type HyperlaneConfig = {
  chain_id: number;
  infura_rpc_url: string;
  hyperlane_deployments: HyperlaneContracts;
  hyperlane_domain: number;
  hyperlane_protocol_fee: number;
  ism: {
    static_message_id_multisig_ism: Ism;
  };
  proxy_admin_address: Address;
  warp_routes: Warproute[];
};

type Warproute = {
  warp_route_type: { erc20_collateral: Address } | string;
  proxy_address: Address;
  symbol: string;
};

type Ism = {
  validators: string[];
  threshold: number;
};

type HyperlaneContracts = {
  static_message_id_multisig_ism_factory: string;
  mailbox: string;
};
