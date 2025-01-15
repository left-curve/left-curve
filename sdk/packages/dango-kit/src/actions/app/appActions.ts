import type { Chain, Client, Signer, Transport } from "@left-curve/types";
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
import {
  type ComputeAddressParameters,
  type ComputeAddressReturnType,
  computeAddress,
} from "./queries/computeAddress.js";

export type AppQueryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  computeAddress(args: ComputeAddressParameters): ComputeAddressReturnType;
};

export type AppMutationActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
> = {
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

export function appQueryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(_client: Client<transport, chain, signer>): AppQueryActions<transport, chain, signer> {
  return {
    computeAddress: (args: ComputeAddressParameters) => computeAddress(args),
  };
}

export function appMutationActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): AppMutationActions<transport, chain, signer> {
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
