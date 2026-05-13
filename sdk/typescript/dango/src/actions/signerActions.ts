import {
  type AccountFactoryMutationActions,
  accountFactoryMutationActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppMutationActions, appMutationActions } from "./app/appActions.js";
import { type DexMutationActions, dexMutationActions } from "./dex/dexActions.js";
import { type PerpsMutationActions, perpsMutationActions } from "./perps/perpsActions.js";

import type { Client } from "../types/client.js";
import type { Signer } from "../types/signer.js";

export type SignerActions = AppMutationActions &
  AccountFactoryMutationActions &
  DexMutationActions &
  PerpsMutationActions;

export function signerActions(client: Client<Signer>): SignerActions {
  return {
    ...appMutationActions(client),
    ...accountFactoryMutationActions(client),
    ...dexMutationActions(client),
    ...perpsMutationActions(client),
  };
}
