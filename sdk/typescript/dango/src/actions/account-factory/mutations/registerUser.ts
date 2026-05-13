import { getAppConfig, simulate } from "../../../index.js";
import { broadcastTxSync } from "../../app/mutations/broadcastTxSync.js";

import { getAction } from "../../index.js";
import type { AppConfig, Client, Key, KeyHash, Signature, Signer } from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/mutations/broadcastTxSync.js";

export type RegisterUserParameters = {
  key: Key;
  keyHash: KeyHash;
  seed: number;
  signature: Signature;
  referrer?: number;
};

export type RegisterUserReturnType = BroadcastTxSyncReturnType;

export type MsgRegisterUser = {
  registerUser: {
    keyHash: KeyHash;
    key: Key;
    seed: number;
    signature: Signature;
    referrer: number | null;
  };
};

export async function registerUser(
  client: Client<Signer | undefined>,
  parameters: RegisterUserParameters,
): RegisterUserReturnType {
  const { keyHash, key, seed, signature, referrer } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getAppConfigAction<AppConfig>({});

  const registerMsg = {
    registerUser: {
      keyHash,
      key,
      seed,
      signature,
      referrer: referrer ?? null,
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
