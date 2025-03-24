import { changeAccount as changeAccountAction } from "./changeAccount.js";
import { refreshAccounts as refreshAccountsAction } from "./refreshAccounts.js";

import type { Account, AccountTypes, KeyHash, Username } from "@left-curve/dango/types";
import type { Chain, ChainId } from "@left-curve/dango/types";

import type { Connector } from "../types/connector.js";
import type { Config } from "../types/store.js";

export type GetAccountReturnType<accounType extends AccountTypes = AccountTypes> =
  | {
      username: Username;
      keyHash: KeyHash;
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
      keyHash: KeyHash | undefined;
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
      keyHash: KeyHash | undefined;
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
      keyHash: undefined;
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
  keyHash: undefined,
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
  const { chainId, connectors, status } = config.state;
  const connectorUId = config.state.current!;
  const connection = connectors.get(connectorUId);

  if (!connection) {
    return disconnected;
  }

  const chain = config.chain;

  const changeAccount = (account: Account) => {
    changeAccountAction(config, { account, connectorUId: connectorUId! });
  };

  const refreshAccounts = async () => {
    refreshAccountsAction(config, { connectorUId });
  };

  const { accounts, connector, username, account: acc, keyHash } = connection;
  const account = acc as Account<accountType>;
  switch (status) {
    case "connected":
      return {
        username,
        keyHash,
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
        keyHash,
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
        keyHash,
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
