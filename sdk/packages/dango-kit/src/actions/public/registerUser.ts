import { simulate } from "../../../../core/src/actions/simulate.js";
import { type BroadcastTxSyncReturnType, broadcastTxSync } from "../signer/broadcastTxSync.js";
import { getAppConfig } from "./getAppConfig.js";

import type {
  Chain,
  Client,
  Credential,
  Hex,
  Key,
  KeyHash,
  Signer,
  Transport,
  Username,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

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

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  const registerMsg = {
    registerUser: {
      username,
      keyHash,
      key,
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
