import type { Account, Chain, ChainId, Config, Connector, Username } from "@leftcurve/types";

export type GetAccountReturnType =
  | {
      username: Username;
      accounts: readonly Account[];
      chain: Chain | undefined;
      chainId: ChainId;
      connector: Connector;
      isConnected: true;
      isConnecting: false;
      isDisconnected: false;
      isReconnecting: false;
      status: "connected";
    }
  | {
      username: Username | undefined;
      accounts: readonly Account[] | undefined;
      chain: Chain | undefined;
      chainId: ChainId | undefined;
      connector: Connector | undefined;
      isConnected: boolean;
      isConnecting: false;
      isDisconnected: false;
      isReconnecting: true;
      status: "reconnecting";
    }
  | {
      username: Username | undefined;
      accounts: readonly Account[] | undefined;
      chain: Chain | undefined;
      chainId: ChainId | undefined;
      connector: Connector | undefined;
      isConnected: false;
      isReconnecting: false;
      isConnecting: true;
      isDisconnected: false;
      status: "connecting";
    }
  | {
      username: undefined;
      accounts: undefined;
      chain: undefined;
      chainId: undefined;
      connector: undefined;
      isConnected: false;
      isReconnecting: false;
      isConnecting: false;
      isDisconnected: true;
      status: "disconnected";
    };

const disconnected = {
  username: undefined,
  accounts: undefined,
  chain: undefined,
  chainId: undefined,
  connector: undefined,
  isConnected: false,
  isConnecting: false,
  isDisconnected: true,
  isReconnecting: false,
  status: "disconnected",
} as const;

export function getAccount<config extends Config>(config: config): GetAccountReturnType {
  const { chainId, connections, connectors, status } = config.state;
  const connectorId = connectors.get(chainId);
  const connection = connections.get(connectorId!);

  if (!connection) {
    return disconnected;
  }

  const chain = config.chains.find((chain) => chain.id === chainId);

  const { accounts, connector, username } = connection;
  switch (status) {
    case "connected":
      return {
        username,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: true,
        isConnecting: false,
        isDisconnected: false,
        isReconnecting: false,
        status,
      };
    case "reconnecting":
      return {
        username,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: !!accounts.length,
        isConnecting: false,
        isDisconnected: false,
        isReconnecting: true,
        status,
      };
    case "connecting":
      return {
        username,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: false,
        isConnecting: true,
        isDisconnected: false,
        isReconnecting: false,
        status,
      };
    case "disconnected":
      return disconnected;
  }
}
