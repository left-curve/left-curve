import type {
  Address,
  Chain,
  Client,
  Credential,
  Hex,
  Key,
  KeyHash,
  Signer,
  Transport,
  Username,
} from "@leftcurve/types";
import { type BroadcastTxSyncReturnType, broadcastTxSync } from "../user/broadcastTxSync.js";
import { getAppConfig } from "./getAppConfig.js";
import { simulate } from "./simulate.js";

export type RegisterUserParameters = {
  username: Username;
  key: Key;
  keyHash: KeyHash;
};

export type RegisterUserReturnType = BroadcastTxSyncReturnType;

export type MsgRegisterUser = {
  registerUser: {
    username: string;
    KeyHash: Hex;
    key: Credential;
  };
};

export async function registerUser<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: RegisterUserParameters,
): RegisterUserReturnType {
  const { username, keyHash, key } = parameters;

  const factoryAddr = await getAppConfig<Address>(client, { key: "account_factory" });

  const registerMsg = {
    registerUser: {
      username,
      keyHash,
      key,
    },
  };

  const executeMsg = {
    execute: {
      contract: factoryAddr,
      msg: registerMsg,
      funds: {},
    },
  };

  const { gasUsed } = await simulate(client, {
    simulate: { sender: factoryAddr, msgs: [executeMsg], data: null },
  });

  const tx = {
    sender: factoryAddr,
    msgs: [executeMsg],
    gasLimit: gasUsed,
    data: null,
    credential: null,
  };

  return await broadcastTxSync(client, { tx });
}
