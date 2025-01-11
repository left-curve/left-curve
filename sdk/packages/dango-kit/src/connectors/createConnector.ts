import type { Chain, CreateConnectorFn, Signer, Transport } from "@left-curve/types";

export function createConnector<
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
  properties extends Record<string, unknown> = Record<string, unknown>,
  chain extends Chain = Chain,
  signer extends Signer = Signer,
  transport extends Transport = Transport,
>(createConnectorFn: CreateConnectorFn<provider, chain, signer, transport, properties>) {
  return createConnectorFn;
}
