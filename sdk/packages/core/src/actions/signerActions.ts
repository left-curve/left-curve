import type { Chain, Client, Signer, Transport, TxParameters } from "@leftcurve/types";

import { type ExecuteParameters, type ExecuteReturnType, execute } from "./signer/execute.js";

import { type MigrateParameters, type MigrateReturnType, migrate } from "./signer/migrate.js";

import { type TransferParameters, type TransferReturnType, transfer } from "./signer/transfer.js";

import {
  type StoreCodeParameters,
  type StoreCodeReturnType,
  storeCode,
} from "./signer/storeCode.js";

import {
  type RegisterAccountParameters,
  type RegisterAccountReturnType,
  registerAccount,
} from "./signer/registerAccount.js";

import {
  type InstantiateParameters,
  type InstantiateReturnType,
  instantiate,
} from "./signer/instantiate.js";

import {
  type StoreCodeAndInstantiateParameters,
  type StoreCodeAndInstantiateReturnType,
  storeCodeAndInstantiate,
} from "./signer/storeCodeAndInstantiate.js";

import {
  type SignAndBroadcastTxParameters,
  type SignAndBroadcastTxReturnType,
  signAndBroadcastTx,
} from "./signer/signAndBroadcastTx.js";

export type SignerActions<
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

export function signerActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): SignerActions<transport, chain, signer> {
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
