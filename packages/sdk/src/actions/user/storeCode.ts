import type { Account, Address, Base64, Chain, Client, Hex, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type StoreCodeParameters = {
  sender: Address;
  code: Base64;
};

export type StoreCodeReturnType = Promise<Hex>;

export async function storeCode<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: StoreCodeParameters): StoreCodeReturnType {
  const { sender, code } = parameters;
  const storeCodeMsg = { upload: { code } };
  return await signAndBroadcastTx(client, { sender, msgs: [storeCodeMsg] });
}
