import { getAppConfig, simulate } from "@left-curve/sdk";
import type { Hex, Transport } from "@left-curve/sdk/types";
import { broadcastTxSync } from "../../app/mutations/broadcastTxSync.js";

import { getAction } from "@left-curve/sdk/actions";
import type {
  AppConfig,
  DangoClient,
  Key,
  KeyHash,
  Signature,
  Signer,
  Username,
} from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";

export type RegisterUserParameters = {
  username: Username;
  key: Key;
  keyHash: KeyHash;
  seed: number;
  signature: Signature;
};

export type RegisterUserReturnType = BroadcastTxSyncReturnType;

export type MsgRegisterUser = {
  registerUser: {
    username: string;
    KeyHash: Hex;
    key: Key;
    seed: number;
    signature: Signature;
  };
};

export async function registerUser<transport extends Transport>(
  client: DangoClient<transport, undefined | Signer>,
  parameters: RegisterUserParameters,
): RegisterUserReturnType {
  const { username, keyHash, key, seed, signature } = parameters;

  const geAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await geAppConfigAction<AppConfig>({});

  const registerMsg = {
    registerUser: {
      username,
      keyHash,
      key,
      seed,
      signature,
    },
  };

  const executeMsg = {
    execute: {
      contract: addresses.accountFactory,
      msg: registerMsg,
      funds: {},
    },
  };

  const simulateAction = getAction(client, simulate, "simulate");

  const { gasUsed } = await simulateAction({
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
