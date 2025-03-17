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
  type GetAccountTypeCodeHashParameters,
  type GetAccountTypeCodeHashReturnType,
  getAccountTypeCodeHash,
} from "./queries/getAccountTypeCodeHash.js";

import {
  type GetAccountTypeCodeHashesParameters,
  type GetAccountTypeCodeHashesReturnType,
  getAccountTypeCodeHashes,
} from "./queries/getAccountTypeCodeHashes.js";

import {
  type GetAccountsByUsernameParameters,
  type GetAccountsByUsernameReturnType,
  getAccountsByUsername,
} from "./queries/getAccountsByUsername.js";

import {
  type GetAllAccountInfoParameters,
  type GetAllAccountInfoReturnType,
  getAllAccountInfo,
} from "./queries/getAllAccountInfo.js";

import {
  type GetDepositParameters,
  type GetDepositReturnType,
  getDeposit,
} from "./queries/getDeposit.js";

import {
  type GetDepositsParameters,
  type GetDepositsReturnType,
  getDeposits,
} from "./queries/getDeposits.js";

import { type GetKeyParameters, type GetKeyReturnType, getKey } from "./queries/getKey.js";

import { type GetKeysParameters, type GetKeysReturnType, getKeys } from "./queries/getKeys.js";

import {
  type GetKeysByUsernameParameters,
  type GetKeysByUsernameReturnType,
  getKeysByUsername,
} from "./queries/getKeysByUsername.js";

import {
  type GetNextAccountAddressParameters,
  type GetNextAccountAddressReturnType,
  getNextAccountAddress,
} from "./queries/getNextAccountAddress.js";

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
  type GetUsersByKeyHashReturnType,
  type GetUsersByKeyhashParameters,
  getUsersByKeyHash,
} from "./queries/getUsersByKeyHash.js";

import {
  type CreateSessionParameters,
  type CreateSessionReturnType,
  createSession,
} from "./mutations/createSession.js";

import type { Chain, DangoClient, Signer } from "../../types/index.js";

export type AccountFactoryQueryActions = {
  getAccountInfo: (args: GetAccountInfoParameters) => GetAccountInfoReturnType;
  getAccountsByUsername: (args: GetAccountsByUsernameParameters) => GetAccountsByUsernameReturnType;
  getAccountSeenNonces: (args: GetAccountSeenNoncesParameters) => GetAccountSeenNoncesReturnType;
  getAccountTypeCodeHash: (
    args: GetAccountTypeCodeHashParameters,
  ) => GetAccountTypeCodeHashReturnType;
  getAccountTypeCodeHashes: (
    args: GetAccountTypeCodeHashesParameters,
  ) => GetAccountTypeCodeHashesReturnType;
  getAllAccountInfo: (args: GetAllAccountInfoParameters) => GetAllAccountInfoReturnType;
  getDeposit: (args: GetDepositParameters) => GetDepositReturnType;
  getDeposits: (args: GetDepositsParameters) => GetDepositsReturnType;
  getKey: (args: GetKeyParameters) => GetKeyReturnType;
  getKeys: (args: GetKeysParameters) => GetKeysReturnType;
  getKeysByUsername: (args: GetKeysByUsernameParameters) => GetKeysByUsernameReturnType;
  getNextAccountAddress: (args: GetNextAccountAddressParameters) => GetNextAccountAddressReturnType;
  getNextAccountIndex: (args: GetNextAccountIndexParameters) => GetNextAccountIndexReturnType;
  getUser: (args: GetUserParameters) => GetUserReturnType;
  getUsersByKeyHash: (args: GetUsersByKeyhashParameters) => GetUsersByKeyHashReturnType;
};

export type AccountFactoryMutationActions = {
  registerUser: (args: RegisterUserParameters) => RegisterUserReturnType;
  updateKey: (args: UpdateKeyParameters) => UpdateKeyReturnType;
  registerAccount: (
    args: RegisterAccountParameters,
    txArgs: TxParameters,
  ) => RegisterUserReturnType;
  createSession: (args: CreateSessionParameters) => CreateSessionReturnType;
};

export function accountFactoryQueryActions<transport extends Transport = Transport>(
  client: Client<transport, Chain, Signer>,
): AccountFactoryQueryActions {
  return {
    getAccountInfo: (args) => getAccountInfo(client, args),
    getAccountsByUsername: (args) => getAccountsByUsername(client, args),
    getAccountSeenNonces: (args) => getAccountSeenNonces(client, args),
    getAccountTypeCodeHash: (args) => getAccountTypeCodeHash(client, args),
    getAccountTypeCodeHashes: (args) => getAccountTypeCodeHashes(client, args),
    getAllAccountInfo: (args) => getAllAccountInfo(client, args),
    getDeposit: (args) => getDeposit(client, args),
    getDeposits: (args) => getDeposits(client, args),
    getKey: (args) => getKey(client, args),
    getKeys: (args) => getKeys(client, args),
    getKeysByUsername: (args) => getKeysByUsername(client, args),
    getNextAccountAddress: (args) => getNextAccountAddress(client, args),
    getNextAccountIndex: (args) => getNextAccountIndex(client, args),
    getUser: (args) => getUser(client, args),
    getUsersByKeyHash: (args) => getUsersByKeyHash(client, args),
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
  };
}
