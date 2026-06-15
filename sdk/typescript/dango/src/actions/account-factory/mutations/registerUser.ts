import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { simulate } from "#actions/app/queries/simulate.js";
import { broadcastTxSync } from "#actions/app/mutations/broadcastTxSync.js";

import type { Client, Key, KeyHash, Signature, Signer } from "@left-curve/types";
import type { BroadcastTxSyncReturnType } from "#actions/app/mutations/broadcastTxSync.js";

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

  const { addresses } = await getAppConfig(client);

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
