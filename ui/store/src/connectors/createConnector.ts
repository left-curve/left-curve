import type { CreateConnectorFn } from "../types/connector.js";

export function createConnector<
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
  properties extends Record<string, unknown> = Record<string, unknown>,
>(createConnectorFn: CreateConnectorFn<provider, properties>) {
  return createConnectorFn;
}
