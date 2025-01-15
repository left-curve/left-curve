import { getAppConfig, simulate } from "@left-curve/sdk";
import type { Chain, Client, Hex, Signer, Transport } from "@left-curve/types";
import { broadcastTxSync } from "../../app/broadcastTxSync.js";

import type { AppConfig, Key, KeyHash, Username } from "../../../types/index.js";
import type { BroadcastTxSyncReturnType } from "../../app/broadcastTxSync.js";

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

  const { addresses } = await getAppConfig<AppConfig>(client);

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
