import { encodeBase64 } from "@left-curve/encoding";
import type {
  Address,
  Base64,
  Client,
  Funds,
  Hex,
  Json,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";

import { getCoinsTypedData } from "@left-curve/utils";
import { computeAddress } from "../../../account/address.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type { DangoClient, Signer } from "../../../types/index.js";

export type StoreCodeAndInstantiateParameters = {
  sender: Address;
  codeHash: Hex;
  msg: Json;
  salt: Uint8Array;
  funds?: Funds;
  code: Base64;
  admin?: Address;
  typedData?: TypedDataParameter;
};

export type StoreCodeAndInstantiateReturnType = Promise<
  [string, Awaited<SignAndBroadcastTxReturnType>]
>;

export async function storeCodeAndInstantiate<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: StoreCodeAndInstantiateParameters,
): StoreCodeAndInstantiateReturnType {
  const { sender, msg, codeHash, funds = {}, salt, code, admin } = parameters;
  const address = computeAddress({ deployer: sender, codeHash, salt });

  const instantiateMsg = {
    instantiate: {
      codeHash,
      msg,
      salt: encodeBase64(salt),
      funds,
      admin: admin,
    },
  };

  const { extraTypes = {}, type = [] } = parameters.typedData || {};

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [
      { name: "instantiate", type: "Instantiate" },
      { name: "upload", type: "Upload" },
    ],
    extraTypes: {
      Instantiate: [
        { name: "codeHash", type: "string" },
        { name: "salt", type: "string" },
        { name: "admin", type: "address" },
        { name: "funds", type: "Funds" },
        { name: "msg", type: "InstantiateAndUploadMessage" },
      ],
      Upload: [{ name: "code", type: "string" }],
      Funds: [...getCoinsTypedData(funds)],
      InstantiateAndUploadMessage: type,
      ...extraTypes,
    },
  };

  const storeCodeMsg = { upload: { code } };

  const txData = await signAndBroadcastTx(client, {
    sender,
    messages: [storeCodeMsg, instantiateMsg],
    typedData,
  });

  return [address, txData];
}
