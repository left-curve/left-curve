import type { Transport } from "@left-curve/dango/types";
import type { CreateConnectorFn } from "../types/connector.js";

export function createConnector<
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
  properties extends Record<string, unknown> = Record<string, unknown>,
  transport extends Transport = Transport,
>(createConnectorFn: CreateConnectorFn<provider, transport, properties>) {
  return createConnectorFn;
}
