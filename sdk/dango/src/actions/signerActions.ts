import {
  type AccountFactoryMutationActions,
  accountFactoryMutationActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppMutationActions, appMutationActions } from "./app/appActions.js";
import { type DexMutationActions, dexMutationActions } from "./dex/dexActions.js";

import type { Transport } from "@left-curve/sdk/types";

import type { DangoClient } from "../types/clients.js";
import type { Signer } from "../types/signer.js";

export type SignerActions = AppMutationActions & AccountFactoryMutationActions & DexMutationActions;

export function signerActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): SignerActions {
  return {
    ...appMutationActions(client),
    ...accountFactoryMutationActions(client),
    ...dexMutationActions(client),
  };
}
