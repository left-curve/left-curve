import { encodeBase64 } from "@leftcurve/encoding";
import type {
  Address,
  Chain,
  Client,
  Funds,
  Hex,
  Json,
  Signer,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@leftcurve/types";
import { getCoinsTypedData } from "@leftcurve/utils";
import { computeAddress } from "../public/computeAddress.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

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

export async function instantiate<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
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
