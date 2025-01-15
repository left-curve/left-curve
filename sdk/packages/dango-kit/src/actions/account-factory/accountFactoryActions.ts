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

import {
  type GetUsersByKeyHashReturnType,
  type GetUsersByKeyhashParameters,
  getUsersByKeyHash,
} from "./queries/getUsersByKeyHash.js";

export type AccountFactoryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  // queries
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
  // mutations
  registerUser: (args: RegisterUserParameters) => RegisterUserReturnType;
};

export function accountFactoryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): AccountFactoryActions<transport, chain, signer> {
  return {
    // queries
    getAccountInfo: (args: GetAccountInfoParameters) => getAccountInfo<chain, signer>(client, args),
    getAccountsByUsername: (args: GetAccountsByUsernameParameters) =>
      getAccountsByUsername<chain, signer>(client, args),
    getAccountSeenNonces: (args: GetAccountSeenNoncesParameters) =>
      getAccountSeenNonces<chain, signer>(client, args),
    getAccountTypeCodeHash: (args: GetAccountTypeCodeHashParameters) =>
      getAccountTypeCodeHash<chain, signer>(client, args),
    getAccountTypeCodeHashes: (args: GetAccountTypeCodeHashesParameters) =>
      getAccountTypeCodeHashes<chain, signer>(client, args),
    getAllAccountInfo: (args: GetAllAccountInfoParameters) =>
      getAllAccountInfo<chain, signer>(client, args),
    getDeposit: (args: GetDepositParameters) => getDeposit<chain, signer>(client, args),
    getDeposits: (args: GetDepositsParameters) => getDeposits<chain, signer>(client, args),
    getKey: (args: GetKeyParameters) => getKey<chain, signer>(client, args),
    getKeys: (args: GetKeysParameters) => getKeys<chain, signer>(client, args),
    getKeysByUsername: (args: GetKeysByUsernameParameters) =>
      getKeysByUsername<chain, signer>(client, args),
    getNextAccountAddress: (args: GetNextAccountAddressParameters) =>
      getNextAccountAddress<chain, signer>(client, args),
    getNextAccountIndex: (args: GetNextAccountIndexParameters) =>
      getNextAccountIndex<chain, signer>(client, args),
    getUser: (args: GetUserParameters) => getUser<chain, signer>(client, args),
    getUsersByKeyHash: (args: GetUsersByKeyhashParameters) =>
      getUsersByKeyHash<chain, signer>(client, args),
    // mutations
    registerUser: (args: RegisterUserParameters) => registerUser<chain, signer>(client, args),
  };
}
