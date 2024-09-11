import type {
  Address,
  Base64,
  Chain,
  Client,
  Hex,
  MessageTypedDataType,
  Signer,
  Transport,
  TypedDataParameter,
} from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type StoreCodeParameters = {
  sender: Address;
  code: Base64;
};

export type StoreCodeReturnType = Promise<Hex>;

export async function storeCode<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: StoreCodeParameters,
): StoreCodeReturnType {
  const { sender, code } = parameters;
  const storeCodeMsg = { upload: { code } };

  const typedData: TypedDataParameter<MessageTypedDataType> = {
    type: [{ name: "upload", type: "Upload" }],
    extraTypes: {
      Upload: [{ name: "code", type: "string" }],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [storeCodeMsg], typedData });
}
