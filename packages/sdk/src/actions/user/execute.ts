import type { Address, Chain, Client, Coins, Hex, Json, Signer, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type ExecuteParameters = {
  sender: Address;
  contract: Address;
  msg: Json;
  funds?: Coins;
  gasLimit?: number;
};

export type ExecuteReturnType = Promise<Hex>;

export async function execute<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: ExecuteParameters,
): ExecuteReturnType {
  const { sender, contract, msg, gasLimit, funds = {} } = parameters;
  const executeMsg = {
    execute: {
      contract,
      msg,
      funds,
    },
  };

  return await signAndBroadcastTx(client, { sender, msgs: [executeMsg], gasLimit });
}
