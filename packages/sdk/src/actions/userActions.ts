import type { Chain, Client, Signer, Transport, TxParameters } from "@leftcurve/types";

import { type ExecuteParameters, type ExecuteReturnType, execute } from "./user/execute";

import { type MigrateParameters, type MigrateReturnType, migrate } from "./user/migrate";

import { type TransferParameters, type TransferReturnType, transfer } from "./user/transfer";

import { type StoreCodeParameters, type StoreCodeReturnType, storeCode } from "./user/storeCode";

import {
  type RegisterAccountParameters,
  type RegisterAccountReturnType,
  registerAccount,
} from "./user/registerAccount";

import {
  type InstantiateParameters,
  type InstantiateReturnType,
  instantiate,
} from "./user/instantiate";

import {
  type StoreCodeAndInstantiateParameters,
  type StoreCodeAndInstantiateReturnType,
  storeCodeAndInstantiate,
} from "./user/storeCodeAndInstantiate";

import {
  type SignAndBroadcastTxParameters,
  type SignAndBroadcastTxReturnType,
  signAndBroadcastTx,
} from "./user/signAndBroadcastTx";

export type UserActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  execute: (args: ExecuteParameters) => ExecuteReturnType;
  migrate: (args: MigrateParameters) => MigrateReturnType;
  transfer: (args: TransferParameters) => TransferReturnType;
  storeCode: (args: StoreCodeParameters) => StoreCodeReturnType;
  instantiate: (args: InstantiateParameters) => InstantiateReturnType;
  registerAccount: (
    args: RegisterAccountParameters,
    txArgs: TxParameters,
  ) => RegisterAccountReturnType;
  storeCodeAndInstantiate: (
    args: StoreCodeAndInstantiateParameters,
  ) => StoreCodeAndInstantiateReturnType;
  signAndBroadcastTx: (args: SignAndBroadcastTxParameters) => SignAndBroadcastTxReturnType;
};

export function userActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): UserActions<transport, chain, signer> {
  return {
    execute: (args: ExecuteParameters) => execute<chain, signer>(client, args),
    migrate: (args: MigrateParameters) => migrate<chain, signer>(client, args),
    transfer: (args: TransferParameters) => transfer<chain, signer>(client, args),
    storeCode: (args: StoreCodeParameters) => storeCode<chain, signer>(client, args),
    instantiate: (args: InstantiateParameters) => instantiate<chain, signer>(client, args),
    registerAccount: (...args) => registerAccount<chain, signer>(client, ...args),
    storeCodeAndInstantiate: (args: StoreCodeAndInstantiateParameters) =>
      storeCodeAndInstantiate<chain, signer>(client, args),
    signAndBroadcastTx: (args: SignAndBroadcastTxParameters) =>
      signAndBroadcastTx<chain, signer>(client, args),
  };
}
