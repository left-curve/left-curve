import { encodeBase64 } from "@left-curve/encoding";
import type {
  Address,
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

export type InstantiateParameters = {
  sender: Address;
  codeHash: Hex;
  msg: Json;
  salt: Uint8Array;
  funds?: Funds;
  admin?: Address;
  gasLimit?: number;
  typedData?: TypedDataParameter;
};

export type InstantiateReturnType = Promise<[string, Awaited<SignAndBroadcastTxReturnType>]>;

export async function instantiate<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: InstantiateParameters,
): InstantiateReturnType {
  const { sender, msg, codeHash, salt, admin, gasLimit, funds = {} } = parameters;
  const address = computeAddress({ deployer: sender, codeHash, salt });

  const instantiateMsg = {
    instantiate: {
      codeHash,
      msg,
      salt: encodeBase64(salt),
      funds,
      admin,
    },
  };

  const { extraTypes = {}, type = [] } = parameters.typedData || {};

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "instantiate", type: "Instantiate" }],
    extraTypes: {
      Instantiate: [
        { name: "codeHash", type: "string" },
        { name: "salt", type: "string" },
        { name: "admin", type: "address" },
        { name: "funds", type: "Funds" },
        { name: "msg", type: "InstantiateMessage" },
      ],
      Funds: [...getCoinsTypedData(funds)],
      InstantiateMessage: type,
      ...extraTypes,
    },
  };

  const txHash = await signAndBroadcastTx(client, {
    sender,
    messages: [instantiateMsg],
    gasLimit,
    typedData,
  });

  return [address, txHash];
}
