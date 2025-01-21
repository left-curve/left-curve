import type { Chain, ChainId } from "@left-curve/types";
import type { Account, AccountTypes, Config, Connector, Username } from "../../types/index.js";
import { changeAccount as changeAccountAction } from "./changeAccount.js";
import { refreshAccounts as refreshAccountsAction } from "./refreshAccounts.js";

export type GetAccountReturnType<accounType extends AccountTypes = AccountTypes> =
  | {
      username: Username;
      account: Account<accounType>;
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
      refreshAccounts: () => Promise<void>;
    }
  | {
      username: Username | undefined;
      account: Account<accounType> | undefined;
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
      refreshAccounts: undefined;
    }
  | {
      username: Username | undefined;
      account: Account<accounType> | undefined;
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
      refreshAccounts: undefined;
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
      refreshAccounts: undefined;
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
  refreshAccounts: undefined,
} as const;

export function getAccount<
  accountType extends AccountTypes = AccountTypes,
  config extends Config = Config,
>(config: config): GetAccountReturnType<accountType> {
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

  const refreshAccounts = async () => {
    refreshAccountsAction(config, { connectorUId });
  };

  const { accounts, connector, username, account: acc } = connection;
  const account = acc as Account<accountType>;
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
        refreshAccounts,
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
        refreshAccounts: undefined,
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
        refreshAccounts: undefined,
      };
    case "disconnected":
      return disconnected;
  }
}
