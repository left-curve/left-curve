import type { CreateConnectorFn } from "@leftcurve/types";

export function createConnector<
  provider,
  signDoc = unknown,
  properties extends Record<string, unknown> = Record<string, unknown>,
  storageItem extends Record<string, unknown> = Record<string, unknown>,
>(createConnectorFn: CreateConnectorFn<provider, signDoc, properties, storageItem>) {
  return createConnectorFn;
}
