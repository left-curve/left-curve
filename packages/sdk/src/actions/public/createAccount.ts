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
import { getAppConfig } from "./getAppConfig";
import { simulate } from "./simulate";

export type CreateAccountParameters = {
  username: Username;
  key: Key;
  keyHash: KeyHash;
};

export type CreateAccountReturnType = Promise<Hex>;

export type MsgRegisterUser = {
  registerUser: {
    username: string;
    KeyHash: Hex;
    key: Credential;
  };
};

export async function createAccount<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: CreateAccountParameters,
): CreateAccountReturnType {
  const { username, keyHash, key } = parameters;

  const accountFactory = await getAppConfig<Address, chain, signer>(client, {
    key: "account_factory",
  });

  const registerMsg = {
    registerUser: {
      username,
      keyHash,
      key,
    },
  };

  const executeMsg = {
    execute: {
      contract: accountFactory,
      msg: registerMsg,
      funds: {},
    },
  };

  const { gasLimit } = await simulate(client, {
    simulate: { sender: accountFactory, msgs: [executeMsg], data: null },
  });

  const tx = {
    sender: accountFactory,
    msgs: [executeMsg],
    gasLimit: gasLimit,
    data: null,
    credential: null,
  };

  return await client.broadcast(tx);
}
