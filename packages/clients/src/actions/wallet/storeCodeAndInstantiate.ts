import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import {
  type Account,
  type AdminOption,
  type Base64,
  type Chain,
  type Client,
  type Coin,
  type Hex,
  type Json,
  type Transport,
  createAddress,
} from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type StoreCodeAndInstantiateParameters = {
  sender: string;
  codeHash: Uint8Array;
  msg: Json;
  salt: Uint8Array;
  funds: Coin;
  code: Base64;
  adminOpt?: AdminOption;
};

export type StoreCodeAndInstantiateReturnType = Promise<[string, Hex]>;

export async function storeCodeAndInstantiate<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: StoreCodeAndInstantiateParameters,
): StoreCodeAndInstantiateReturnType {
  const { sender, msg, codeHash, funds, salt, code, adminOpt } = parameters;
  const address = createAddress(sender, codeHash, salt);
  // TODO: handle adminOpt
  const instantiateMsg = {
    instantiate: {
      codeHash: encodeHex(codeHash),
      msg,
      salt: encodeBase64(salt),
      funds,
      admin: undefined,
    },
  };
  const storeCodeMsg = { upload: { code } };

  const txHash = await signAndBroadcastTx(client, { sender, msgs: [storeCodeMsg, instantiateMsg] });

  return [address, txHash];
}
