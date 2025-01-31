import type { Address, Base64, Transport } from "@left-curve/sdk/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

export type StoreCodeParameters = {
  sender: Address;
  code: Base64;
};

export type StoreCodeReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function storeCode<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: StoreCodeParameters,
): StoreCodeReturnType {
  const { sender, code } = parameters;
  const storeCodeMsg = { upload: { code } };

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "upload", type: "Upload" }],
    extraTypes: {
      Upload: [{ name: "code", type: "string" }],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [storeCodeMsg], typedData });
}
