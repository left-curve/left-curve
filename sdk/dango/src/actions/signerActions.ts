import {
  type AccountFactoryMutationActions,
  accountFactoryMutationActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppMutationActions, appMutationActions } from "./app/appActions.js";

import type { Transport } from "@left-curve/sdk/types";

import type { DangoClient, Signer } from "../types/index.js";

export type SignerActions = AppMutationActions & AccountFactoryMutationActions;

export function signerActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): SignerActions {
  return {
    ...appMutationActions(client),
    ...accountFactoryMutationActions(client),
  };
}
