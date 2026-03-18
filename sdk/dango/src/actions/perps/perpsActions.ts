import type { Client, Transport } from "@left-curve/sdk/types";

import {
  type GetPerpsUserStateParameters,
  type GetPerpsUserStateReturnType,
  getPerpsUserState,
} from "./queries/getUserState.js";

export type PerpsQueryActions = {
  getPerpsUserState: (args: GetPerpsUserStateParameters) => GetPerpsUserStateReturnType;
};

export function perpsQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): PerpsQueryActions {
  return {
    getPerpsUserState: (args) => getPerpsUserState(client, args),
  };
}
