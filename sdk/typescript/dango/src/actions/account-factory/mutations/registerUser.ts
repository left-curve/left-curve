import { getAppConfig, simulate } from "@left-curve/sdk";
import type { Transport } from "@left-curve/sdk/types";
import { broadcastTxSync } from "../../app/mutations/broadcastTxSync.js";

import { getAction } from "@left-curve/sdk/actions";
import type {
  AppConfig,
  DangoClient,
  Key,
  KeyHash,
  Signature,
  Signer,
} from "../../../types/index.js";
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

export async function registerUser<transport extends Transport>(
  client: DangoClient<transport, undefined | Signer>,
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
