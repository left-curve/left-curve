import { getConnector } from "./getConnector.js";
import { getConnectorClient } from "./getConnectorClient.js";

import type { Address, UID } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";
import { toAccount } from "@left-curve/dango";

export type RefreshAccountsParameters = {
  userIndex: number;
  connectorUId?: UID;
};

export type RefreshAccountsReturnType = void;

export async function refreshAccounts<config extends Config>(
  config: config,
  parameters: RefreshAccountsParameters,
): Promise<RefreshAccountsReturnType> {
  const { userIndex } = parameters;

  const connectorUId = (() => {
    if (parameters.connectorUId) return parameters.connectorUId;
    const connector = getConnector(config);
    return connector.uid;
  })();

  const client = await getConnectorClient(config, { connectorUId: connectorUId });

  const user = await client.getUser({ userIndexOrName: { index: userIndex } });

  config.setState((x) => {
    const connector = x.connectors.get(connectorUId);
    if (!connector) return x;

    const updatedAccounts = Object.entries(user.accounts).map(([accountIndex, address]) =>
      toAccount({ user, accountIndex: Number(accountIndex), address: address as Address }),
    );

    const currentAccountAddress = connector.account?.address;
    const updatedAccount = currentAccountAddress
      ? updatedAccounts.find((acc) => acc.address === currentAccountAddress)
      : updatedAccounts[0];

    return {
      ...x,
      connectors: new Map(x.connectors).set(connectorUId, {
        ...connector,
        accounts: updatedAccounts,
        account: updatedAccount ?? connector.account,
      }),
    };
  });
}
