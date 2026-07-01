import { type UseQueryResult, useQuery } from "@tanstack/react-query";

import {
  type ConnectorWalletClient,
  getConnectorWalletClient,
  isEvmProviderConnector,
} from "../actions/getConnectorWalletClient.js";

import type { Connector } from "../types/connector.js";
import type { Chain as ViemChain } from "viem";

export type UseConnectorWalletClientParameters<
  chain extends ViemChain | undefined = ViemChain | undefined,
> = {
  connector?: Connector;
  chain?: chain;
  enabled?: boolean;
};

export type UseConnectorWalletClientReturnType<
  chain extends ViemChain | undefined = ViemChain | undefined,
> = UseQueryResult<ConnectorWalletClient<chain>, Error>;

export function useConnectorWalletClient<chain extends ViemChain>(
  parameters: UseConnectorWalletClientParameters<chain> & { chain: chain },
): UseConnectorWalletClientReturnType<chain>;
export function useConnectorWalletClient(
  parameters?: UseConnectorWalletClientParameters<undefined>,
): UseConnectorWalletClientReturnType<undefined>;
export function useConnectorWalletClient(
  parameters: UseConnectorWalletClientParameters = {},
): UseConnectorWalletClientReturnType {
  const { chain, connector, enabled = true } = parameters;
  const isEnabled = enabled && isEvmProviderConnector(connector);

  return useQuery<ConnectorWalletClient, Error>({
    enabled: isEnabled,
    queryKey: ["connector_wallet_client", connector?.uid, connector?.id, chain?.id],
    queryFn: async () => {
      if (!connector) throw new Error("Connector not found");
      if (chain) return getConnectorWalletClient({ connector, chain });
      return getConnectorWalletClient({ connector });
    },
  });
}
