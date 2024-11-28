import type { SignerClient } from "@left-curve/sdk/clients";
import type { Config, ConnectorUId } from "@left-curve/types";
import { getConnector } from "./getConnector.js";

export type GetConnectorClientParameters = {
  connectorUId?: ConnectorUId;
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
