import type { Client, Transport } from "@left-curve/types";
import type { Chain, Signer } from "../types/index.js";
import {
  type AccountFactoryMutationActions,
  accountFactoryMutationActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppMutationActions, appMutationActions } from "./app/appActions.js";

export type SignerActions = AppMutationActions & AccountFactoryMutationActions;

export function signerActions<transport extends Transport = Transport>(
  client: Client<transport, Chain, Signer>,
): SignerActions {
  return {
    ...appMutationActions(client),
    ...accountFactoryMutationActions(client),
  };
}
