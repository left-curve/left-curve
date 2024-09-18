import type { Config } from "@leftcurve/types";

export type GetBlockExplorerParameters = {
  explorerName?: string;
};

export type GetBlockExplorerReturnType = {
  name: string;
  getTxUrl: (txHash: string) => string;
  getAccountUrl: (address: string) => string;
};

export type GetBlockExplorerErrorType = Error;

export function getBlockExplorer<config extends Config>(
  config: config,
  parameters: GetBlockExplorerParameters,
): GetBlockExplorerReturnType {
  const { explorerName } = parameters;

  const chain = config.chains.find((chain) => chain.id === config.state.chainId);

  if (!chain) throw new Error("Chain not found");
  if (!chain.blockExplorers) throw new Error("Block explorers not found");

  const { name, txPage, accountPage } = chain.blockExplorers[explorerName || "default"];

  return {
    name,
    getTxUrl: (txHash: string) => txPage.replace("${tx_hash}", txHash),
    getAccountUrl: (address: string) => accountPage.replace("${address}", address),
  };
}
