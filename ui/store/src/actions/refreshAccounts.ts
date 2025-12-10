import { getConnector } from "./getConnector.js";
import { getConnectorClient } from "./getConnectorClient.js";

import type { UserIndexAndName } from "@left-curve/dango/types";
import type { Address, UID } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";
import { toAccount } from "@left-curve/dango";

export type RefreshAccountsParameters = {
  userIndexAndName: UserIndexAndName;
  connectorUId?: UID;
};

export type RefreshAccountsReturnType = void;

export async function refreshAccounts<config extends Config>(
  config: config,
  parameters: RefreshAccountsParameters,
): Promise<RefreshAccountsReturnType> {
  const { userIndexAndName } = parameters;

  const connectorUId = (() => {
    if (parameters.connectorUId) return parameters.connectorUId;
    const connector = getConnector(config);
    return connector.uid;
  })();

  const client = await getConnectorClient(config, { connectorUId: connectorUId });

  const accountsInfo = await client.getAccountsByUsername({ userIndexOrName: userIndexAndName });

  config.setState((x) => {
    const connector = x.connectors.get(connectorUId);
    if (!connector) return x;
    return {
      ...x,
      connectors: new Map(x.connectors).set(connectorUId, {
        ...connector,
        accounts: Object.entries(accountsInfo).map(([address, accountInfo]) =>
          toAccount({ userIndexAndName, address: address as Address, info: accountInfo }),
        ),
      }),
    };
  });
}
