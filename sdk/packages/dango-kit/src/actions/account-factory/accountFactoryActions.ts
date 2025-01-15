import type { Chain, Client, Signer, Transport } from "@left-curve/types";

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
  type GetUsersByKeyHashReturnType,
  type GetUsersByKeyhashParameters,
  getUsersByKeyHash,
} from "./queries/getUsersByKeyHash.js";

export type AccountFactoryQueryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
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

export type AccountFactoryMutationActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  registerUser: (args: RegisterUserParameters) => RegisterUserReturnType;
  registerAccount: (args: RegisterAccountParameters) => RegisterUserReturnType;
};

export function accountFactoryQueryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): AccountFactoryQueryActions<transport, chain, signer> {
  return {
    // queries
    getAccountInfo: (args) => getAccountInfo<chain, signer>(client, args),
    getAccountsByUsername: (args) => getAccountsByUsername<chain, signer>(client, args),
    getAccountSeenNonces: (args) => getAccountSeenNonces<chain, signer>(client, args),
    getAccountTypeCodeHash: (args) => getAccountTypeCodeHash<chain, signer>(client, args),
    getAccountTypeCodeHashes: (args) => getAccountTypeCodeHashes<chain, signer>(client, args),
    getAllAccountInfo: (args) => getAllAccountInfo<chain, signer>(client, args),
    getDeposit: (args) => getDeposit<chain, signer>(client, args),
    getDeposits: (args) => getDeposits<chain, signer>(client, args),
    getKey: (args) => getKey<chain, signer>(client, args),
    getKeys: (args) => getKeys<chain, signer>(client, args),
    getKeysByUsername: (args) => getKeysByUsername<chain, signer>(client, args),
    getNextAccountAddress: (args) => getNextAccountAddress<chain, signer>(client, args),
    getNextAccountIndex: (args) => getNextAccountIndex<chain, signer>(client, args),
    getUser: (args) => getUser<chain, signer>(client, args),
    getUsersByKeyHash: (args) => getUsersByKeyHash<chain, signer>(client, args),
  };
}

export function accountFactoryMutationActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(
  client: Client<transport, chain, signer>,
): AccountFactoryMutationActions<transport, chain, signer> {
  return {
    registerUser: (args) => registerUser<chain, signer>(client, args),
    registerAccount: (args) => registerAccount<chain, signer>(client, args),
  };
}
