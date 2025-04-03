import type { Client, Transport } from "@left-curve/sdk/types";
import {
  type BroadcastTxSyncParameters,
  type BroadcastTxSyncReturnType,
  broadcastTxSync,
} from "./mutations/broadcastTxSync.js";
import { type ExecuteParameters, type ExecuteReturnType, execute } from "./mutations/execute.js";
import {
  type InstantiateParameters,
  type InstantiateReturnType,
  instantiate,
} from "./mutations/instantiate.js";
import { type MigrateParameters, type MigrateReturnType, migrate } from "./mutations/migrate.js";
import {
  type SignAndBroadcastTxParameters,
  type SignAndBroadcastTxReturnType,
  signAndBroadcastTx,
} from "./mutations/signAndBroadcastTx.js";
import {
  type StoreCodeParameters,
  type StoreCodeReturnType,
  storeCode,
} from "./mutations/storeCode.js";
import {
  type StoreCodeAndInstantiateParameters,
  type StoreCodeAndInstantiateReturnType,
  storeCodeAndInstantiate,
} from "./mutations/storeCodeAndInstantiate.js";
import {
  type TransferParameters,
  type TransferReturnType,
  transfer,
} from "./mutations/transfer.js";

import type {
  QueryAppParameters,
  QueryAppReturnType,
  QueryTxParameters,
  QueryTxReturnType,
  SimulateParameters,
  SimulateReturnType,
} from "@left-curve/sdk/actions";
import type { AppConfig, DangoClient, Signer } from "#types/index.js";
import { getAppConfig } from "./queries/getAppConfig.js";
import { queryApp } from "./queries/queryApp.js";
import { type QueryStatusReturnType, queryStatus } from "./queries/queryStatus.js";
import { queryTx } from "./queries/queryTx.js";
import { simulate } from "./queries/simulate.js";

export type AppQueryActions = {
  getAppConfig: () => Promise<AppConfig>;
  queryTx(args: QueryTxParameters): QueryTxReturnType;
  queryApp(args: QueryAppParameters): QueryAppReturnType;
  queryStatus(): QueryStatusReturnType;
  simulate(args: SimulateParameters): SimulateReturnType;
};

export type AppMutationActions = {
  broadcastTxSync(args: BroadcastTxSyncParameters): BroadcastTxSyncReturnType;
  execute(args: ExecuteParameters): ExecuteReturnType;
  instantiate(args: InstantiateParameters): InstantiateReturnType;
  migrate(args: MigrateParameters): MigrateReturnType;
  signAndBroadcastTx(args: SignAndBroadcastTxParameters): SignAndBroadcastTxReturnType;
  storeCode(args: StoreCodeParameters): StoreCodeReturnType;
  storeCodeAndInstantiate(
    args: StoreCodeAndInstantiateParameters,
  ): StoreCodeAndInstantiateReturnType;
  transfer(args: TransferParameters): TransferReturnType;
};

export function appQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): AppQueryActions {
  return {
    getAppConfig: () => getAppConfig(client),
    queryTx: (args) => queryTx(client, args),
    queryApp: (args) => queryApp(client, args),
    queryStatus: () => queryStatus(client),
    simulate: (args) => simulate(client, args),
  };
}

export function appMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): AppMutationActions {
  return {
    broadcastTxSync: (args) => broadcastTxSync(client, args),
    execute: (args) => execute(client, args),
    instantiate: (args) => instantiate(client, args),
    migrate: (args) => migrate(client, args),
    signAndBroadcastTx: (args) => signAndBroadcastTx(client, args),
    storeCode: (args) => storeCode(client, args),
    storeCodeAndInstantiate: (args) => storeCodeAndInstantiate(client, args),
    transfer: (args) => transfer(client, args),
  };
}
