import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";
import type { Signer } from "../../types/signer.js";

import {
  type GetPerpsUserStateParameters,
  type GetPerpsUserStateReturnType,
  getPerpsUserState,
} from "./queries/getUserState.js";

import {
  type GetPerpsOrdersByUserParameters,
  type GetPerpsOrdersByUserReturnType,
  getPerpsOrdersByUser,
} from "./queries/getOrdersByUser.js";

import {
  type GetPerpsLiquidityDepthParameters,
  type GetPerpsLiquidityDepthReturnType,
  getPerpsLiquidityDepth,
} from "./queries/getLiquidityDepth.js";

import {
  type GetPerpsPairParamParameters,
  type GetPerpsPairParamReturnType,
  getPerpsPairParam,
} from "./queries/getPairParam.js";

import {
  type GetPerpsPairParamsParameters,
  type GetPerpsPairParamsReturnType,
  getPerpsPairParams,
} from "./queries/getPairParams.js";

import {
  type GetPerpsParamParameters,
  type GetPerpsParamReturnType,
  getPerpsParam,
} from "./queries/getParam.js";

import {
  type QueryPerpsCandlesParameters,
  type QueryPerpsCandlesReturnType,
  queryPerpsCandles,
} from "./queries/perpsCandles.js";

import {
  type QueryPerpsEventsParameters,
  type QueryPerpsEventsReturnType,
  queryPerpsEvents,
} from "./queries/perpsEvents.js";

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

import {
  type SubmitPerpsOrderParameters,
  type SubmitPerpsOrderReturnType,
  submitPerpsOrder,
} from "./mutations/submitOrder.js";

import {
  type CancelPerpsOrderParameters,
  type CancelPerpsOrderReturnType,
  cancelPerpsOrder,
} from "./mutations/cancelOrder.js";

import {
  type SetReferralParameters,
  type SetReferralReturnType,
  setReferral,
} from "./mutations/setReferral.js";

import {
  type SetFeeShareRatioParameters,
  type SetFeeShareRatioReturnType,
  setFeeShareRatio,
} from "./mutations/setFeeShareRatio.js";

export type PerpsQueryActions = {
  getPerpsUserState: (args: GetPerpsUserStateParameters) => GetPerpsUserStateReturnType;
  getPerpsOrdersByUser: (args: GetPerpsOrdersByUserParameters) => GetPerpsOrdersByUserReturnType;
  getPerpsLiquidityDepth: (
    args: GetPerpsLiquidityDepthParameters,
  ) => GetPerpsLiquidityDepthReturnType;
  getPerpsPairParam: (args: GetPerpsPairParamParameters) => GetPerpsPairParamReturnType;
  getPerpsPairParams: (args?: GetPerpsPairParamsParameters) => GetPerpsPairParamsReturnType;
  getPerpsParam: (args?: GetPerpsParamParameters) => GetPerpsParamReturnType;
  queryPerpsCandles: (args: QueryPerpsCandlesParameters) => QueryPerpsCandlesReturnType;
  queryPerpsEvents: (args: QueryPerpsEventsParameters) => QueryPerpsEventsReturnType;
};

export function perpsQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): PerpsQueryActions {
  return {
    getPerpsUserState: (args) => getPerpsUserState(client, args),
    getPerpsOrdersByUser: (args) => getPerpsOrdersByUser(client, args),
    getPerpsLiquidityDepth: (args) => getPerpsLiquidityDepth(client, args),
    getPerpsPairParam: (args) => getPerpsPairParam(client, args),
    getPerpsPairParams: (args) => getPerpsPairParams(client, args),
    getPerpsParam: (args) => getPerpsParam(client, args),
    queryPerpsCandles: (args) => queryPerpsCandles(client, args),
    queryPerpsEvents: (args) => queryPerpsEvents(client, args),
  };
}

export type PerpsMutationActions = {
  depositMargin: (args: DepositMarginParameters) => DepositMarginReturnType;
  withdrawMargin: (args: WithdrawMarginParameters) => WithdrawMarginReturnType;
  submitPerpsOrder: (args: SubmitPerpsOrderParameters) => SubmitPerpsOrderReturnType;
  cancelPerpsOrder: (args: CancelPerpsOrderParameters) => CancelPerpsOrderReturnType;
  setReferral: (args: SetReferralParameters) => SetReferralReturnType;
  setFeeShareRatio: (args: SetFeeShareRatioParameters) => SetFeeShareRatioReturnType;
};

export function perpsMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): PerpsMutationActions {
  return {
    depositMargin: (args) => depositMargin(client, args),
    withdrawMargin: (args) => withdrawMargin(client, args),
    submitPerpsOrder: (args) => submitPerpsOrder(client, args),
    cancelPerpsOrder: (args) => cancelPerpsOrder(client, args),
    setReferral: (args) => setReferral(client, args),
    setFeeShareRatio: (args) => setFeeShareRatio(client, args),
  };
}
