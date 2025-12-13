import { getConnector } from "./getConnector.js";
import { getConnectorClient } from "./getConnectorClient.js";

import type { Address, UID, UserStatus } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";

export type RefreshUserStatusParameters = {
  connectorUId?: UID;
  address?: Address;
};

export type RefreshUserStatusReturnType = void;

export async function refreshUserStatus<config extends Config>(
  config: config,
  parameters: RefreshUserStatusParameters = {},
): Promise<RefreshUserStatusReturnType> {
  const { connectorUId, address } = parameters;
  const connector = getConnector(config, { connectorUId });
  const connection = config.state.connectors.get(connector.uid);

  if (!connection) return;

  const client = await getConnectorClient(config, { connectorUId: connector.uid });

  const accountAddress = address ?? connection.account?.address ?? connection.accounts[0]?.address;

  if (!accountAddress) {
    config.setState((x) => ({ ...x, userStatus: undefined }));
    return;
  }

  const userStatus: UserStatus = await client.getAccountStatus({ address: accountAddress });
  config.setState((x) => ({ ...x, userStatus }));
}
