import type { Transport } from "@left-curve/sdk/types";
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

import type { DangoClient, Signer } from "../../types/index.js";

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
