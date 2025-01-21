import type { AccountTypes, Address, Config, ConnectorUId, Username } from "@left-curve/types";
import { getConnector } from "./getConnector.js";
import { getConnectorClient } from "./getConnectorClient.js";

export type RefreshAccountsParameters = {
  username?: Username;
  connectorUId?: ConnectorUId;
};

export type RefreshAccountsReturnType = void;

export async function refreshAccounts<config extends Config>(
  config: config,
  parameters: RefreshAccountsParameters = {},
): Promise<RefreshAccountsReturnType> {
  const connectorUId = (() => {
    if (parameters.connectorUId) return parameters.connectorUId;
    const connector = getConnector(config);
    return connector.uid;
  })();

  const client = await getConnectorClient(config, { connectorUId: connectorUId });

  const username = (() => {
    if (parameters.username) return parameters.username;
    if (client.username) return client.username;
    throw new Error("Username not provided");
  })();

  const accounts = await client.getAccountsByUsername({ username });

  config.setState((x) => {
    const connection = x.connections.get(connectorUId);
    if (!connection) return x;
    return {
      ...x,
      connections: new Map(x.connections).set(connectorUId, {
        ...connection,
        accounts: Object.entries(accounts).map(([address, accountInfo]) => {
          const { index, params } = accountInfo;
          const type = Object.keys(params)[0] as AccountTypes;
          return {
            index,
            params,
            address: address as Address,
            username,
            type: type,
          };
        }),
      }),
    };
  });
}
