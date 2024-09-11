import type { Chain, CreateConnectorFn, SignDoc, Signer, Transport } from "@leftcurve/types";

export function createConnector<
  provider = undefined,
  properties extends Record<string, unknown> = Record<string, unknown>,
  chain extends Chain = Chain,
  signer extends Signer = Signer,
  signDoc extends SignDoc = SignDoc,
  transport extends Transport = Transport,
>(createConnectorFn: CreateConnectorFn<provider, chain, signer, signDoc, transport, properties>) {
  return createConnectorFn;
}
