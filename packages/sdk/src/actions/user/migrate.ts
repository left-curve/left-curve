import type { Address, Chain, Client, Hex, Json, Signer, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type MigrateParameters = {
  sender: Address;
  contract: Address;
  newCodeHash: Hex;
  msg: Json;
};

export type MigrateReturnType = Promise<Hex>;

export async function migrate<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: MigrateParameters,
): MigrateReturnType {
  const { sender, contract, msg, newCodeHash } = parameters;
  const migrateMsg = {
    migrate: {
      contract,
      msg,
      newCodeHash,
    },
  };

  return await signAndBroadcastTx(client, { sender, msgs: [migrateMsg] });
}
