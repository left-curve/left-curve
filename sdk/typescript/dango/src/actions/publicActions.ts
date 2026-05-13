import type { Client } from "../types/client.js";

import { type GrugActions, grugActions } from "./grugActions.js";

import {
  type AccountFactoryQueryActions,
  accountFactoryQueryActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppQueryActions, appQueryActions } from "./app/appActions.js";
import { type DexQueryActions, dexQueryActions } from "./dex/dexActions.js";
import { type IndexerActions, indexerActions } from "./indexer/indexerActions.js";
import { type OracleQueryActions, oracleQueryActions } from "./oracle/oracleActions.js";
import { type PerpsQueryActions, perpsQueryActions } from "./perps/perpsActions.js";

export type PublicActions = Omit<GrugActions, "queryStatus" | "getAppConfig"> &
  AppQueryActions &
  AccountFactoryQueryActions &
  IndexerActions &
  OracleQueryActions &
  DexQueryActions &
  PerpsQueryActions;

export function publicActions(client: Client): PublicActions {
  return {
    ...grugActions(client),
    ...appQueryActions(client),
    ...indexerActions(client),
    ...oracleQueryActions(client),
    ...accountFactoryQueryActions(client),
    ...dexQueryActions(client),
    ...perpsQueryActions(client),
  };
}
