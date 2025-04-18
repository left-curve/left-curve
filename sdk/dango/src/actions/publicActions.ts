import { type GrugActions, grugActions } from "@left-curve/sdk";
import type { Client, Transport } from "@left-curve/sdk/types";

import {
  type AccountFactoryQueryActions,
  accountFactoryQueryActions,
} from "./account-factory/accountFactoryActions.js";
import { type AppQueryActions, appQueryActions } from "./app/appActions.js";
import { type IndexerActions, indexerActions } from "./indexer/indexerActions.js";
import { type OracleQueryActions, oracleQueryActions } from "./oracle/oracleActions.js";

export type PublicActions = Omit<GrugActions, "queryStatus" | "getAppConfig"> &
  AppQueryActions &
  AccountFactoryQueryActions &
  IndexerActions &
  OracleQueryActions;

export function publicActions<transport extends Transport = Transport>(
  client: Client<transport>,
): PublicActions {
  return {
    ...grugActions(client),
    ...appQueryActions(client),
    ...indexerActions(client),
    ...oracleQueryActions(client),
    ...accountFactoryQueryActions(client),
  };
}
