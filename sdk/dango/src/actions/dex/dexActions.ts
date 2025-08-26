import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "../../types/clients.js";
import type { Signer } from "../../types/signer.js";

import { type GetPairsParameters, type GetPairsReturnType, getPairs } from "./queries/getPairs.js";

import { type GetPairParameters, type GetPairReturnType, getPair } from "./queries/getPair.js";

import {
  type SimulateSwapExactAmountOutParameters,
  type SimulateSwapExactAmountOutReturnType,
  simulateSwapExactAmountOut,
} from "./queries/simulateSwapExactAmountOut.js";

import {
  type SimulateSwapExactAmountInParameters,
  type SimulateSwapExactAmountInReturnType,
  simulateSwapExactAmountIn,
} from "./queries/simulateSwapExactAmountIn.js";

import {
  type ProvideLiquidityParameters,
  type ProvideLiquidityReturnType,
  provideLiquidity,
} from "./mutations/provideLiquidity.js";

import {
  type WithdrawLiquidityParameters,
  type WithdrawLiquidityReturnType,
  withdrawLiquidity,
} from "./mutations/withdrawLiquidity.js";

import {
  type SwapExactAmountOutParameters,
  type SwapExactAmountOutReturnType,
  swapExactAmountOut,
} from "./mutations/swapExactAmountOut.js";

import {
  type SwapExactAmountInParameters,
  type SwapExactAmountInReturnType,
  swapExactAmountIn,
} from "./mutations/swapExactAmountIn.js";

import {
  type BatchUpdateOrdersParameters,
  type BatchUpdateOrdersReturnType,
  batchUpdateOrders,
} from "./mutations/batchUpdateOrders.js";

import {
  type OrdersByUserParameters,
  type OrdersByUserReturnType,
  ordersByUser,
} from "./queries/ordersByUser.js";

import {
  type QueryCandlesParameters,
  type QueryCandlesReturnType,
  queryCandles,
} from "./queries/candles.js";

import {
  type SimulateWithdrawLiquidityParameters,
  type SimulateWithdrawLiquidityReturnType,
  simulateWithdrawLiquidity,
} from "./queries/simulateWithdrawLiquidity.js";

import {
  type QueryTradesParameters,
  type QueryTradesReturnType,
  queryTrades,
} from "./queries/trades.js";

export type DexQueryActions = {
  getPairs: (args?: GetPairsParameters) => GetPairsReturnType;
  getPair: (args: GetPairParameters) => GetPairReturnType;
  ordersByUser: (args: OrdersByUserParameters) => OrdersByUserReturnType;
  queryCandles: (args: QueryCandlesParameters) => QueryCandlesReturnType;
  queryTrades: (args: QueryTradesParameters) => QueryTradesReturnType;
  simulateWithdrawLiquidity: (
    args: SimulateWithdrawLiquidityParameters,
  ) => SimulateWithdrawLiquidityReturnType;
  simulateSwapExactAmountOut: (
    args: SimulateSwapExactAmountOutParameters,
  ) => SimulateSwapExactAmountOutReturnType;
  simulateSwapExactAmountIn: (
    args: SimulateSwapExactAmountInParameters,
  ) => SimulateSwapExactAmountInReturnType;
};

export function dexQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): DexQueryActions {
  return {
    getPairs: (args) => getPairs(client, args),
    getPair: (args) => getPair(client, args),
    ordersByUser: (args) => ordersByUser(client, args),
    queryCandles: (args) => queryCandles(client, args),
    queryTrades: (args) => queryTrades(client, args),
    simulateWithdrawLiquidity: (args) => simulateWithdrawLiquidity(client, args),
    simulateSwapExactAmountOut: (args) => simulateSwapExactAmountOut(client, args),
    simulateSwapExactAmountIn: (args) => simulateSwapExactAmountIn(client, args),
  };
}

export type DexMutationActions = {
  batchUpdateOrders: (args: BatchUpdateOrdersParameters) => BatchUpdateOrdersReturnType;
  swapExactAmountIn: (args: SwapExactAmountInParameters) => SwapExactAmountInReturnType;
  swapExactAmountOut: (args: SwapExactAmountOutParameters) => SwapExactAmountOutReturnType;
  provideLiquidity: (args: ProvideLiquidityParameters) => ProvideLiquidityReturnType;
  withdrawLiquidity: (args: WithdrawLiquidityParameters) => WithdrawLiquidityReturnType;
};

export function dexMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): DexMutationActions {
  return {
    batchUpdateOrders: (args) => batchUpdateOrders(client, args),
    swapExactAmountIn: (args) => swapExactAmountIn(client, args),
    swapExactAmountOut: (args) => swapExactAmountOut(client, args),
    provideLiquidity: (args) => provideLiquidity(client, args),
    withdrawLiquidity: (args) => withdrawLiquidity(client, args),
  };
}
