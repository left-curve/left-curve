import { encodeBase64 } from "@leftcurve/encoding";
import type {
  Account,
  Address,
  Chain,
  Client,
  Coins,
  Hex,
  Json,
  Transport,
} from "@leftcurve/types";
import { computeAddress } from "../public/computeAddress";
import { signAndBroadcastTx } from "./signAndBroadcastTx";

export type InstantiateParameters = {
  sender: Address;
  codeHash: Hex;
  msg: Json;
  salt: Uint8Array;
  funds?: Coins;
  admin?: Address;
  gasLimit?: number;
};

export type InstantiateReturnType = Promise<[string, Hex]>;

export async function instantiate<
  chain extends Chain | undefined,
  account extends Account | undefined,
>(
  client: Client<Transport, chain, account>,
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

  const txHash = await signAndBroadcastTx(client, { sender, msgs: [instantiateMsg], gasLimit });

  return [address, txHash];
}
