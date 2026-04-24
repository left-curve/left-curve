import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";
import type { Signer } from "../../types/signer.js";

import {
  type GetPerpsUserStateParameters,
  type GetPerpsUserStateReturnType,
  getPerpsUserState,
} from "./queries/getUserState.js";

import {
  type GetPerpsUserStateExtendedParameters,
  type GetPerpsUserStateExtendedReturnType,
  getPerpsUserStateExtended,
} from "./queries/getUserStateExtended.js";

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
  type GetPerpsPairStatsParameters,
  type GetPerpsPairStatsReturnType,
  getPerpsPairStats,
} from "./queries/getPerpsPairStats.js";

import {
  type GetAllPerpsPairStatsReturnType,
  getAllPerpsPairStats,
} from "./queries/getAllPerpsPairStats.js";

import {
  type GetPerpsPairStateParameters,
  type GetPerpsPairStateReturnType,
  getPerpsPairState,
} from "./queries/getPerpsPairState.js";

import {
  type GetPerpsStateParameters,
  type GetPerpsStateReturnType,
  getPerpsState,
} from "./queries/getPerpsState.js";

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

import {
  type GetPerpsVaultStateParameters,
  type GetPerpsVaultStateReturnType,
  getPerpsVaultState,
} from "./queries/getVaultState.js";

import {
  type GetFeeRateOverrideParameters,
  type GetFeeRateOverrideReturnType,
  getFeeRateOverride,
} from "./queries/getFeeRateOverride.js";

import {
  type VaultAddLiquidityParameters,
  type VaultAddLiquidityReturnType,
  vaultAddLiquidity,
} from "./mutations/vaultAddLiquidity.js";

import {
  type VaultRemoveLiquidityParameters,
  type VaultRemoveLiquidityReturnType,
  vaultRemoveLiquidity,
} from "./mutations/vaultRemoveLiquidity.js";

import {
  type SubmitConditionalOrderParameters,
  type SubmitConditionalOrderReturnType,
  submitConditionalOrder,
} from "./mutations/submitConditionalOrder.js";

import {
  type SubmitConditionalOrdersParameters,
  type SubmitConditionalOrdersReturnType,
  submitConditionalOrders,
} from "./mutations/submitConditionalOrders.js";

import {
  type CancelConditionalOrderParameters,
  type CancelConditionalOrderReturnType,
  cancelConditionalOrder,
} from "./mutations/cancelConditionalOrder.js";

export type PerpsQueryActions = {
  getPerpsUserState: (args: GetPerpsUserStateParameters) => GetPerpsUserStateReturnType;
  getPerpsUserStateExtended: (
    args: GetPerpsUserStateExtendedParameters,
  ) => GetPerpsUserStateExtendedReturnType;
  getPerpsOrdersByUser: (args: GetPerpsOrdersByUserParameters) => GetPerpsOrdersByUserReturnType;
  getPerpsLiquidityDepth: (
    args: GetPerpsLiquidityDepthParameters,
  ) => GetPerpsLiquidityDepthReturnType;
  getPerpsPairParam: (args: GetPerpsPairParamParameters) => GetPerpsPairParamReturnType;
  getPerpsPairParams: (args?: GetPerpsPairParamsParameters) => GetPerpsPairParamsReturnType;
  getPerpsParam: (args?: GetPerpsParamParameters) => GetPerpsParamReturnType;
  queryPerpsCandles: (args: QueryPerpsCandlesParameters) => QueryPerpsCandlesReturnType;
  queryPerpsEvents: (args: QueryPerpsEventsParameters) => QueryPerpsEventsReturnType;
  getPerpsPairStats: (args: GetPerpsPairStatsParameters) => GetPerpsPairStatsReturnType;
  getAllPerpsPairStats: () => GetAllPerpsPairStatsReturnType;
  getPerpsPairState: (args: GetPerpsPairStateParameters) => GetPerpsPairStateReturnType;
  getPerpsState: (args?: GetPerpsStateParameters) => GetPerpsStateReturnType;
  getPerpsVaultState: (args?: GetPerpsVaultStateParameters) => GetPerpsVaultStateReturnType;
  getFeeRateOverride: (args: GetFeeRateOverrideParameters) => GetFeeRateOverrideReturnType;
};

export function perpsQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): PerpsQueryActions {
  return {
    getPerpsUserState: (args) => getPerpsUserState(client, args),
    getPerpsUserStateExtended: (args) => getPerpsUserStateExtended(client, args),
    getPerpsOrdersByUser: (args) => getPerpsOrdersByUser(client, args),
    getPerpsLiquidityDepth: (args) => getPerpsLiquidityDepth(client, args),
    getPerpsPairParam: (args) => getPerpsPairParam(client, args),
    getPerpsPairParams: (args) => getPerpsPairParams(client, args),
    getPerpsParam: (args) => getPerpsParam(client, args),
    queryPerpsCandles: (args) => queryPerpsCandles(client, args),
    queryPerpsEvents: (args) => queryPerpsEvents(client, args),
    getPerpsPairStats: (args) => getPerpsPairStats(client, args),
    getAllPerpsPairStats: () => getAllPerpsPairStats(client),
    getPerpsPairState: (args) => getPerpsPairState(client, args),
    getPerpsState: (args) => getPerpsState(client, args),
    getPerpsVaultState: (args) => getPerpsVaultState(client, args),
    getFeeRateOverride: (args) => getFeeRateOverride(client, args),
  };
}

export type PerpsMutationActions = {
  depositMargin: (args: DepositMarginParameters) => DepositMarginReturnType;
  withdrawMargin: (args: WithdrawMarginParameters) => WithdrawMarginReturnType;
  submitPerpsOrder: (args: SubmitPerpsOrderParameters) => SubmitPerpsOrderReturnType;
  cancelPerpsOrder: (args: CancelPerpsOrderParameters) => CancelPerpsOrderReturnType;
  setReferral: (args: SetReferralParameters) => SetReferralReturnType;
  setFeeShareRatio: (args: SetFeeShareRatioParameters) => SetFeeShareRatioReturnType;
  vaultAddLiquidity: (args: VaultAddLiquidityParameters) => VaultAddLiquidityReturnType;
  vaultRemoveLiquidity: (args: VaultRemoveLiquidityParameters) => VaultRemoveLiquidityReturnType;
  submitConditionalOrder: (
    args: SubmitConditionalOrderParameters,
  ) => SubmitConditionalOrderReturnType;
  submitConditionalOrders: (
    args: SubmitConditionalOrdersParameters,
  ) => SubmitConditionalOrdersReturnType;
  cancelConditionalOrder: (
    args: CancelConditionalOrderParameters,
  ) => CancelConditionalOrderReturnType;
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
    vaultAddLiquidity: (args) => vaultAddLiquidity(client, args),
    vaultRemoveLiquidity: (args) => vaultRemoveLiquidity(client, args),
    submitConditionalOrder: (args) => submitConditionalOrder(client, args),
    submitConditionalOrders: (args) => submitConditionalOrders(client, args),
    cancelConditionalOrder: (args) => cancelConditionalOrder(client, args),
  };
}
