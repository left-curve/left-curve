import { encodeBase64 } from "@leftcurve/encoding";
import type {
  Address,
  Base64,
  Chain,
  Client,
  Funds,
  Hex,
  Json,
  MessageTypedDataType,
  Signer,
  Transport,
  TypedDataParameter,
} from "@leftcurve/types";

import { getCoinsTypedData } from "@leftcurve/utils";
import { computeAddress } from "../public/computeAddress";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx";

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

export async function storeCodeAndInstantiate<
  chain extends Chain | undefined,
  signer extends Signer,
>(
  client: Client<Transport, chain, signer>,
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

  const typedData: TypedDataParameter<MessageTypedDataType> = {
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
