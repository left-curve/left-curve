import type { Account, Chain, Client, Coin, Hex, Json, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type ExecuteParameters = {
  sender: string;
  contract: string;
  msg: Json;
  funds: Coin;
};

export type ExecuteReturnType = Promise<Hex>;

export async function execute<chain extends Chain | undefined, account extends Account | undefined>(
  client: Client<Transport, chain, account>,
  parameters: ExecuteParameters,
): ExecuteReturnType {
  const { sender, contract, msg, funds } = parameters;
  const executeMsg = {
    execute: {
      contract,
      msg,
      funds,
    },
  };

  return await signAndBroadcastTx(client, { sender, msgs: [executeMsg] });
}
