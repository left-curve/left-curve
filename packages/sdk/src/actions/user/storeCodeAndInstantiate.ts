import { encodeBase64 } from "@leftcurve/encoding";
import type {
  Account,
  Address,
  Base64,
  Chain,
  Client,
  Coins,
  Hex,
  Json,
  Transport,
} from "@leftcurve/types";

import { computeAddress } from "../public/computeAddress";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type StoreCodeAndInstantiateParameters = {
  sender: Address;
  codeHash: Hex;
  msg: Json;
  salt: Uint8Array;
  funds?: Coins;
  code: Base64;
  admin?: Address;
};

export type StoreCodeAndInstantiateReturnType = Promise<[string, Hex]>;

export async function storeCodeAndInstantiate<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
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
  const storeCodeMsg = { upload: { code } };

  const txHash = await signAndBroadcastTx(client, { sender, msgs: [storeCodeMsg, instantiateMsg] });

  return [address, txHash];
}
