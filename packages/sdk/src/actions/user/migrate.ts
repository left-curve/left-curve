import type { Account, Chain, Client, Hex, Json, Transport } from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type MigrateParameters = {
  sender: string;
  contract: string;
  newCodeHash: Hex;
  msg: Json;
};

export type MigrateReturnType = Promise<Hex>;

export async function migrate<chain extends Chain | undefined, account extends Account | undefined>(
  client: Client<Transport, chain, account>,
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
