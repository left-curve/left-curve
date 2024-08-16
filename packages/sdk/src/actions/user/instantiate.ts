import { encodeBase64, encodeHex } from "@leftcurve/encoding";
import type {
  Account,
  Address,
  AdminOption,
  Chain,
  Client,
  Coin,
  Hex,
  Json,
  Transport,
} from "@leftcurve/types";
import { predictAddress } from "../public/predictAddress";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type InstantiateParameters = {
  sender: Address;
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
  const address = predictAddress({ deployer: sender, codeHash, salt });
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
