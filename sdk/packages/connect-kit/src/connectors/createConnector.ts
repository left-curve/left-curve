import type { Chain, CreateConnectorFn, SignDoc, Signer, Transport } from "@left-curve/types";

export function createConnector<
  provider extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
  properties extends Record<string, unknown> = Record<string, unknown>,
  chain extends Chain = Chain,
  signer extends Signer = Signer,
  signDoc extends SignDoc = SignDoc,
  transport extends Transport = Transport,
>(createConnectorFn: CreateConnectorFn<provider, chain, signer, signDoc, transport, properties>) {
  return createConnectorFn;
}
