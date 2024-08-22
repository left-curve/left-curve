import type { Account, Address, Chain, Client, Coins, Hex, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type TransferParameters = {
  sender: Address;
  to: Address;
  coins: Coins;
};

export type TransferReturnType = Promise<Hex>;

export async function transfer<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(client: Client<Transport, chain, account>, parameters: TransferParameters): TransferReturnType {
  const { sender, to, coins } = parameters;
  const transferMsg = { transfer: { to, coins } };
  return await signAndBroadcastTx(client, { sender, msgs: [transferMsg] });
}
