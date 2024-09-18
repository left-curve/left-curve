import type { Account, Chain, ChainId, Config, Connector, Username } from "@leftcurve/types";
import { changeAccount as changeAccountAction } from "./changeAccount";

export type GetAccountReturnType =
  | {
      username: Username;
      account: Account;
      accounts: readonly Account[];
      chain: Chain | undefined;
      chainId: ChainId;
      connector: Connector;
      isConnected: true;
      isConnecting: false;
      isDisconnected: false;
      isReconnecting: false;
      status: "connected";
      changeAccount: (account: Account) => void;
    }
  | {
      username: Username | undefined;
      account: Account | undefined;
      accounts: readonly Account[] | undefined;
      chain: Chain | undefined;
      chainId: ChainId | undefined;
      connector: Connector | undefined;
      isConnected: boolean;
      isConnecting: false;
      isDisconnected: false;
      isReconnecting: true;
      status: "reconnecting";
      changeAccount: undefined;
    }
  | {
      username: Username | undefined;
      account: Account | undefined;
      accounts: readonly Account[] | undefined;
      chain: Chain | undefined;
      chainId: ChainId | undefined;
      connector: Connector | undefined;
      isConnected: false;
      isReconnecting: false;
      isConnecting: true;
      isDisconnected: false;
      status: "connecting";
      changeAccount: undefined;
    }
  | {
      username: undefined;
      account: undefined;
      accounts: undefined;
      chain: undefined;
      chainId: undefined;
      connector: undefined;
      isConnected: false;
      isReconnecting: false;
      isConnecting: false;
      isDisconnected: true;
      status: "disconnected";
      changeAccount: undefined;
    };

const disconnected = {
  username: undefined,
  account: undefined,
  accounts: undefined,
  chain: undefined,
  chainId: undefined,
  connector: undefined,
  isConnected: false,
  isConnecting: false,
  isDisconnected: true,
  isReconnecting: false,
  status: "disconnected",
  changeAccount: undefined,
} as const;

export function getAccount<config extends Config>(config: config): GetAccountReturnType {
  const { chainId, connections, connectors, status } = config.state;
  const connectorUId = connectors.get(chainId);
  const connection = connections.get(connectorUId!);

  if (!connection) {
    return disconnected;
  }

  const chain = config.chains.find((chain) => chain.id === chainId);

  const changeAccount = (account: Account) => {
    changeAccountAction(config, { account, connectorUId: connectorUId! });
  };

  const { accounts, connector, username, account } = connection;
  switch (status) {
    case "connected":
      return {
        username,
        account,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: true,
        isConnecting: false,
        isDisconnected: false,
        isReconnecting: false,
        status,
        changeAccount,
      };
    case "reconnecting":
      return {
        username,
        account,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: false,
        isConnecting: false,
        isDisconnected: false,
        isReconnecting: true,
        status,
        changeAccount: undefined,
      };
    case "connecting":
      return {
        username,
        account,
        accounts,
        chain,
        chainId,
        connector,
        isConnected: false,
        isConnecting: true,
        isDisconnected: false,
        isReconnecting: false,
        status,
        changeAccount: undefined,
      };
    case "disconnected":
      return disconnected;
  }
}
