import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient } from "#types/clients.js";
import type { Signer } from "#types/signer.js";

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

export type DexQueryActions = {
  getPairs: (args?: GetPairsParameters) => GetPairsReturnType;
  getPair: (args: GetPairParameters) => GetPairReturnType;
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
    simulateSwapExactAmountOut: (args) => simulateSwapExactAmountOut(client, args),
    simulateSwapExactAmountIn: (args) => simulateSwapExactAmountIn(client, args),
  };
}

export type DexMutationActions = {
  swapExactAmountIn: (args: SwapExactAmountInParameters) => SwapExactAmountInReturnType;
  swapExactAmountOut: (args: SwapExactAmountOutParameters) => SwapExactAmountOutReturnType;
  provideLiquidity: (args: ProvideLiquidityParameters) => ProvideLiquidityReturnType;
  withdrawLiquidity: (args: WithdrawLiquidityParameters) => WithdrawLiquidityReturnType;
};

export function dexMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): DexMutationActions {
  return {
    swapExactAmountIn: (args) => swapExactAmountIn(client, args),
    swapExactAmountOut: (args) => swapExactAmountOut(client, args),
    provideLiquidity: (args) => provideLiquidity(client, args),
    withdrawLiquidity: (args) => withdrawLiquidity(client, args),
  };
}
