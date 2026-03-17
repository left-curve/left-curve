import type { Client, Transport, TxParameters } from "@left-curve/sdk/types";

import {
  type RegisterUserParameters,
  type RegisterUserReturnType,
  registerUser,
} from "./mutations/registerUser.js";

import {
  type GetAccountInfoParameters,
  type GetAccountInfoReturnType,
  getAccountInfo,
} from "./queries/getAccountInfo.js";

import {
  type GetAccountSeenNoncesParameters,
  type GetAccountSeenNoncesReturnType,
  getAccountSeenNonces,
} from "./queries/getAccountSeenNonces.js";

import {
  type GetCodeHashParameters,
  type GetCodeHashReturnType,
  getCodeHash,
} from "./queries/getCodeHash.js";

import {
  type GetAllAccountInfoParameters,
  type GetAllAccountInfoReturnType,
  getAllAccountInfo,
} from "./queries/getAllAccountInfo.js";

import {
  type GetNextAccountIndexParameters,
  type GetNextAccountIndexReturnType,
  getNextAccountIndex,
} from "./queries/getNextAccountIndex.js";

import { type GetUserParameters, type GetUserReturnType, getUser } from "./queries/getUser.js";

import { type RegisterAccountParameters, registerAccount } from "./mutations/registerAccount.js";

import {
  type UpdateKeyParameters,
  type UpdateKeyReturnType,
  updateKey,
} from "./mutations/updateKey.js";

import {
  type CreateSessionParameters,
  type CreateSessionReturnType,
  createSession,
} from "./mutations/createSession.js";

import type { DangoClient } from "../../types/clients.js";
import type { Signer } from "../../types/signer.js";
import {
  forgotUsername,
  type ForgotUsernameParameters,
  type ForgotUsernameReturnType,
} from "./queries/forgotUsername.js";

import {
  getUserKeys,
  type GetUserKeysParameters,
  type GetUserKeysReturnType,
} from "./queries/getUserKeys.js";

import {
  updateUsername,
  type UpdateUsernameParameters,
  type UpdateUsernameReturnType,
} from "./mutations/updateUsername.js";

import {
  getAccountStatus,
  type GetAccountStatusParameters,
  type GetAccountStatusReturnType,
} from "./queries/getAccountStatus.js";

export type AccountFactoryQueryActions = {
  forgotUsername: (args: ForgotUsernameParameters) => ForgotUsernameReturnType;
  getAccountInfo: (args: GetAccountInfoParameters) => GetAccountInfoReturnType;
  getAccountSeenNonces: (args: GetAccountSeenNoncesParameters) => GetAccountSeenNoncesReturnType;
  getCodeHash: (args?: GetCodeHashParameters) => GetCodeHashReturnType;
  getAllAccountInfo: (args: GetAllAccountInfoParameters) => GetAllAccountInfoReturnType;
  getNextAccountIndex: (args: GetNextAccountIndexParameters) => GetNextAccountIndexReturnType;
  getUser: (args: GetUserParameters) => GetUserReturnType;
  getUserKeys: (args: GetUserKeysParameters) => GetUserKeysReturnType;
  getAccountStatus: (args: GetAccountStatusParameters) => GetAccountStatusReturnType;
};

export type AccountFactoryMutationActions = {
  registerUser: (args: RegisterUserParameters) => RegisterUserReturnType;
  updateKey: (args: UpdateKeyParameters) => UpdateKeyReturnType;
  registerAccount: (
    args: RegisterAccountParameters,
    txArgs?: TxParameters,
  ) => RegisterUserReturnType;
  createSession: (args: CreateSessionParameters) => CreateSessionReturnType;
  updateUsername: (args: UpdateUsernameParameters) => UpdateUsernameReturnType;
};

export function accountFactoryQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): AccountFactoryQueryActions {
  return {
    forgotUsername: (args) => forgotUsername(client, args),
    getAccountInfo: (args) => getAccountInfo(client, args),
    getAccountSeenNonces: (args) => getAccountSeenNonces(client, args),
    getCodeHash: (args) => getCodeHash(client, args),
    getAllAccountInfo: (args) => getAllAccountInfo(client, args),
    getNextAccountIndex: (args) => getNextAccountIndex(client, args),
    getUser: (args) => getUser(client, args),
    getUserKeys: (args) => getUserKeys(client, args),
    getAccountStatus: (args) => getAccountStatus(client, args),
  };
}

export function accountFactoryMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): AccountFactoryMutationActions {
  return {
    registerUser: (...args) => registerUser(client, ...args),
    updateKey: (...args) => updateKey(client, ...args),
    registerAccount: (...args) => registerAccount(client, ...args),
    createSession: (...args) => createSession(client, ...args),
    updateUsername: (...args) => updateUsername(client, ...args),
  };
}
