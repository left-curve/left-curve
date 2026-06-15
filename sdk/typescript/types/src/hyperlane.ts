import type { Address } from "./index.js";

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
  evm: Record<string, HyperlaneEvmChainConfig>;
};

export type HyperlaneEvmChainConfig = {
  chainId: number;
  domain: number;
  estimatedTime: string;
  name: string;
  order: number;
  protocolFee: number;
  rpcUrl: string;
  contracts: HyperlaneContracts;
  ism: {
    staticMessageIdMultisigIsm: Ism;
  };
  routes: HyperlaneWarpRoute[];
};

export type HyperlaneWarpRoute = {
  type: "erc20Collateral" | "native";
  symbol: string;
  tokenAddress: Address | "native";
  routerAddress: Address;
  implementationAddress: Address;
};

type Ism = {
  validators: string[];
  threshold: number;
};

type HyperlaneContracts = {
  mailbox: Address;
  proxyAdmin: Address;
  staticMessageIdMultisigIsmFactory: Address;
};
