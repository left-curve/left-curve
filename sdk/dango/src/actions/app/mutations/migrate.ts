import type { Address, Hex, Json, Transport } from "@left-curve/sdk/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

export type MigrateParameters = {
  sender: Address;
  contract: Address;
  newCodeHash: Hex;
  msg: Json;
  typedData?: TypedDataParameter;
};

export type MigrateReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function migrate<transport extends Transport>(
  client: DangoClient<transport, Signer>,
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

  const { extraTypes = {}, type = [] } = parameters.typedData || {};

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "migrate", type: "Migrate" }],
    extraTypes: {
      Migrate: [
        { name: "contract", type: "address" },
        { name: "new_code_hash", type: "string" },
        { name: "msg", type: "MigrateMessage" },
      ],
      MigrateMessage: type,
      ...extraTypes,
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [migrateMsg], typedData });
}
