import type { UID } from "@left-curve/types";
import type { SignerClient } from "../../clients/signerClient.js";
import type { Config } from "../../types/index.js";
import { getConnector } from "./getConnector.js";

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
