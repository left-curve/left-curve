import { getConnector } from "./getConnector.js";

import type { SignerClient } from "@left-curve/dango/types";
import type { UID } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";

export type GetConnectorClientParameters = {
  connectorUId?: UID;
};

export type GetConnectorClientReturnType = SignerClient;

export type GetConnectorClientErrorType = Error;

export async function getConnectorClient<config extends Config>(
  config: config,
  parameters: GetConnectorClientParameters = {},
): Promise<GetConnectorClientReturnType> {
  const { connectorUId } = parameters;
  const connector = getConnector(config, { connectorUId });

  return await connector.getClient();
}
