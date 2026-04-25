import { encodeBase64, encodeUtf8 } from "@left-curve/sdk/encoding";
import type { Address, Funds, Hex, Json, Transport } from "@left-curve/sdk/types";
import { computeAddress } from "../../../account/address.js";
import { getCoinsTypedData } from "../../../utils/typedData.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

export type InstantiateParameters = {
  sender: Address;
  codeHash: Hex;
  msg: Json;
  salt: Uint8Array | string;
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
  const { sender, msg, codeHash, salt: _salt_, admin, gasLimit, funds = {} } = parameters;

  const salt = typeof _salt_ === "string" ? encodeUtf8(_salt_) : _salt_;

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
        { name: "code_hash", type: "string" },
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
