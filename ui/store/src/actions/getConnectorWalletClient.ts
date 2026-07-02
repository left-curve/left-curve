import {
  createWalletClient,
  custom,
  type Address,
  type Chain as ViemChain,
  type CustomTransport,
  type JsonRpcAccount,
  type WalletClient,
} from "viem";

import { ConnectorTypes, type Connector } from "../types/connector.js";

import type { EIP1193Provider } from "../types/eip1193.js";

export type ConnectorEip1193Provider = EIP1193Provider & Record<string, unknown>;

export type EvmProviderConnector = Connector & {
  getProvider(
    parameters?: { chainId?: string | undefined } | undefined,
  ): Promise<ConnectorEip1193Provider>;
};

export type ConnectorWalletClient<chain extends ViemChain | undefined = ViemChain | undefined> =
  WalletClient<CustomTransport, chain, JsonRpcAccount<Address>>;

export type GetConnectorWalletClientParameters<
  chain extends ViemChain | undefined = ViemChain | undefined,
> = {
  connector: Connector;
  chain?: chain;
};

export function isEvmProviderConnector(
  connector: Connector | null | undefined,
): connector is EvmProviderConnector {
  return (
    !!connector &&
    (connector.type === ConnectorTypes.EIP1193 || connector.type === ConnectorTypes.Privy) &&
    typeof (connector as { getProvider?: unknown }).getProvider === "function"
  );
}

export function getConnectorWalletClient<chain extends ViemChain>(
  parameters: GetConnectorWalletClientParameters<chain> & { chain: chain },
): Promise<ConnectorWalletClient<chain>>;
export function getConnectorWalletClient(
  parameters: GetConnectorWalletClientParameters<undefined>,
): Promise<ConnectorWalletClient<undefined>>;
export async function getConnectorWalletClient({
  connector,
  chain,
}: GetConnectorWalletClientParameters) {
  if (!isEvmProviderConnector(connector)) {
    throw new Error("Connector does not expose an EVM provider");
  }

  const provider = (await connector.getProvider()) as ConnectorEip1193Provider;
  const [evmAddress] = await provider.request({ method: "eth_requestAccounts" });

  if (!evmAddress) throw new Error("No EVM account found");

  if (chain) {
    return createWalletClient({
      chain,
      transport: custom(provider),
      account: evmAddress as Address,
    });
  }

  return createWalletClient({
    transport: custom(provider),
    account: evmAddress as Address,
  });
}
