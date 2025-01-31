import { type GrugActions, grugActions } from "@left-curve/sdk";
import type { Client, Transport } from "@left-curve/sdk/types";
import type { Chain, Signer } from "../types/index.js";
import {
  type AccountFactoryQueryActions,
  accountFactoryQueryActions,
} from "./account-factory/accountFactoryActions.js";

export type PublicActions<transport extends Transport = Transport> = GrugActions<
  transport,
  Chain,
  undefined
> &
  AccountFactoryQueryActions;

export function publicActions<transport extends Transport = Transport>(
  client: Client<transport, Chain, Signer>,
): PublicActions<transport> {
  return {
    ...grugActions(client),
    ...accountFactoryQueryActions(client),
  };
}
