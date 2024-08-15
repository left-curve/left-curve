import type { Account, Chain, Client, Transport } from "@leftcurve/types";

import { type ExecuteParameters, type ExecuteReturnType, execute } from "./wallet/execute";

import { type MigrateParameters, type MigrateReturnType, migrate } from "./wallet/migrate";

import { type TransferParameters, type TransferReturnType, transfer } from "./wallet/transfer";

import { type StoreCodeParameters, type StoreCodeReturnType, storeCode } from "./wallet/storeCode";

import {
  type InstantiateParameters,
  type InstantiateReturnType,
  instantiate,
} from "./wallet/instantiate";

import {
  type StoreCodeAndInstantiateParameters,
  type StoreCodeAndInstantiateReturnType,
  storeCodeAndInstantiate,
} from "./wallet/storeCodeAndInstantiate";

import {
  type SignAndBroadcastTxParameters,
  type SignAndBroadcastTxReturnType,
  signAndBroadcastTx,
} from "./wallet/signAndBroadcastTx";

export type WalletActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
> = {
  execute: (args: ExecuteParameters) => ExecuteReturnType;
  migrate: (args: MigrateParameters) => MigrateReturnType;
  transfer: (args: TransferParameters) => TransferReturnType;
  storeCode: (args: StoreCodeParameters) => StoreCodeReturnType;
  instantiate: (args: InstantiateParameters) => InstantiateReturnType;
  storeCodeAndInstantiate: (
    args: StoreCodeAndInstantiateParameters,
  ) => StoreCodeAndInstantiateReturnType;
  signAndBroadcastTx: (args: SignAndBroadcastTxParameters) => SignAndBroadcastTxReturnType;
};

export function walletActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain | undefined,
  account extends Account | undefined = Account | undefined,
>(client: Client<transport, chain, account>): WalletActions<transport, chain, account> {
  return {
    execute: (args: ExecuteParameters) => execute<chain, account>(client, args),
    migrate: (args: MigrateParameters) => migrate<chain, account>(client, args),
    transfer: (args: TransferParameters) => transfer<chain, account>(client, args),
    storeCode: (args: StoreCodeParameters) => storeCode<chain, account>(client, args),
    instantiate: (args: InstantiateParameters) => instantiate<chain, account>(client, args),
    storeCodeAndInstantiate: (args: StoreCodeAndInstantiateParameters) =>
      storeCodeAndInstantiate<chain, account>(client, args),
    signAndBroadcastTx: (args: SignAndBroadcastTxParameters) =>
      signAndBroadcastTx<chain, account>(client, args),
  };
}
