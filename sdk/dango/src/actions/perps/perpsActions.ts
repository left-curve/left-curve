import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";
import type { Signer } from "../../types/signer.js";

import {
  type GetPerpsUserStateParameters,
  type GetPerpsUserStateReturnType,
  getPerpsUserState,
} from "./queries/getUserState.js";

import {
  type DepositMarginParameters,
  type DepositMarginReturnType,
  depositMargin,
} from "./mutations/depositMargin.js";

import {
  type WithdrawMarginParameters,
  type WithdrawMarginReturnType,
  withdrawMargin,
} from "./mutations/withdrawMargin.js";

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

export type PerpsMutationActions = {
  depositMargin: (args: DepositMarginParameters) => DepositMarginReturnType;
  withdrawMargin: (args: WithdrawMarginParameters) => WithdrawMarginReturnType;
};

export function perpsMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): PerpsMutationActions {
  return {
    depositMargin: (args) => depositMargin(client, args),
    withdrawMargin: (args) => withdrawMargin(client, args),
  };
}
