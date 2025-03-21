import { type GrugActions, grugActions } from "@left-curve/sdk";
import type { Transport } from "@left-curve/sdk/types";
import type { Chain, DangoClient } from "../types/index.js";
import {
  type AccountFactoryQueryActions,
  accountFactoryQueryActions,
} from "./account-factory/accountFactoryActions.js";
import { type IndexerActions, indexerActions } from "./indexer/indexerActions.js";

export type PublicActions = GrugActions & AccountFactoryQueryActions & IndexerActions;

export function publicActions<transport extends Transport = Transport>(
  client: DangoClient<transport>,
): PublicActions {
  return {
    ...grugActions(client),
    ...indexerActions(client),
    ...accountFactoryQueryActions(client),
  };
}
