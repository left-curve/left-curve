import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import {
  type Account,
  type AdminOption,
  type Chain,
  type Client,
  type Coin,
  type Hex,
  type Json,
  type Transport,
  createAddress,
} from "@leftcurve/types";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type InstantiateParameters = {
  sender: string;
  codeHash: Uint8Array;
  msg: Json;
  salt: Uint8Array;
  funds: Coin;
  adminOpt?: AdminOption;
};

export type InstantiateReturnType = Promise<[string, Hex]>;

export async function instantiate<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
  parameters: InstantiateParameters,
): InstantiateReturnType {
  const { sender, msg, codeHash, funds, salt, adminOpt } = parameters;
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

  const txHash = await signAndBroadcastTx(client, { sender, msgs: [instantiateMsg] });

  return [address, txHash];
}
