import type { Account, Chain, Client, Coin, Hex, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type TransferParameters = {
  sender: string;
  to: string;
  coins: Coin;
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
