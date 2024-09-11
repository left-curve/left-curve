import type { CreateConnectorFn, SignDoc } from "@leftcurve/types";

export function createConnector<
  provider = undefined,
  signDoc extends SignDoc = SignDoc,
  properties extends Record<string, unknown> = Record<string, unknown>,
>(createConnectorFn: CreateConnectorFn<provider, signDoc, properties>) {
  return createConnectorFn;
}
