import { getAppConfig, simulate } from "@left-curve/sdk";
import type { Hex, Transport } from "@left-curve/sdk/types";
import { broadcastTxSync } from "../../app/mutations/broadcastTxSync.js";

import type {
  AppConfig,
  DangoClient,
  Key,
  KeyHash,
  Signer,
  Username,
} from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";

export type RegisterUserParameters = {
  username: Username;
  key: Key;
  keyHash: KeyHash;
  secret: number;
};

export type RegisterUserReturnType = BroadcastTxSyncReturnType;

export type MsgRegisterUser = {
  registerUser: {
    username: string;
    KeyHash: Hex;
    key: Credential;
  };
};

export async function registerUser<transport extends Transport>(
  client: DangoClient<transport, undefined | Signer>,
  parameters: RegisterUserParameters,
): RegisterUserReturnType {
  const { username, keyHash, key, secret } = parameters;

  const { addresses } = await getAppConfig<AppConfig>(client);

  const registerMsg = {
    registerUser: {
      username,
      keyHash,
      key,
      secret,
    },
  };

  const executeMsg = {
    execute: {
      contract: addresses.accountFactory,
      msg: registerMsg,
      funds: {},
    },
  };

  const { gasUsed } = await simulate(client, {
    simulate: { sender: addresses.accountFactory, msgs: [executeMsg], data: null },
  });

  const tx = {
    sender: addresses.accountFactory,
    msgs: [executeMsg],
    gasLimit: gasUsed,
    data: null,
    credential: null,
  };

  return await broadcastTxSync(client, { tx });
}
