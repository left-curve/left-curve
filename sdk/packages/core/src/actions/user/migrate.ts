import type {
  Address,
  Chain,
  Client,
  Hex,
  Json,
  Signer,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@leftcurve/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

export type MigrateParameters = {
  sender: Address;
  contract: Address;
  newCodeHash: Hex;
  msg: Json;
  typedData?: TypedDataParameter;
};

export type MigrateReturnType = Promise<SignAndBroadcastTxReturnType>;

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

  const { extraTypes = {}, type = [] } = parameters.typedData || {};

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "migrate", type: "Migrate" }],
    extraTypes: {
      Migrate: [
        { name: "contract", type: "address" },
        { name: "newCodeHash", type: "string" },
        { name: "msg", type: "MigrateMessage" },
      ],
      MigrateMessage: type,
      ...extraTypes,
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [migrateMsg], typedData });
}
