import type { Chain, Client, Signer, Transport, TxParameters } from "@leftcurve/types";

import {
  type CreatePoolParameters,
  type CreatePoolReturnType,
  createPool,
} from "./mutations/createPool.js";

import {
  type ProvideLiquidityParameters,
  type ProvideLiquidityReturnType,
  provideLiquidity,
} from "./mutations/provideLiquidity.js";

import {
  type SwapCoinsParameters,
  type SwapCoinsReturnType,
  swapCoins,
} from "./mutations/swapCoins.js";

import {
  type WithdrawLiquidityParameters,
  type WithdrawLiquidityReturnType,
  withdrawLiquidity,
} from "./mutations/withdrawLiquidity.js";

import {
  type GetAllPoolsParameters,
  type GetAllPoolsReturnType,
  getAllPools,
} from "./queries/getAllPools.js";

import {
  type GetAmmConfigParameters,
  type GetAmmConfigReturnType,
  getAmmConfig,
} from "./queries/getAmmConfig.js";

import {
  type SimulateSwapParameters,
  type SimulateSwapReturnType,
  simulateSwap,
} from "./queries/simulateSwap.js";

import { type GetPoolParameters, type GetPoolReturnType, getPool } from "./queries/getPool.js";

export type AmmActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  // queries
  getPool: (args: GetPoolParameters) => GetPoolReturnType;
  getAllPools: (args: GetAllPoolsParameters) => GetAllPoolsReturnType;
  getAmmConfig: (args: GetAmmConfigParameters) => GetAmmConfigReturnType;
  simulateSwap: (args: SimulateSwapParameters) => SimulateSwapReturnType;

  // mutations
  createPool: (args: CreatePoolParameters, txParameters: TxParameters) => CreatePoolReturnType;
  provideLiquidity: (
    args: ProvideLiquidityParameters,
    txParameters: TxParameters,
  ) => ProvideLiquidityReturnType;
  withdrawLiquidity: (
    args: WithdrawLiquidityParameters,
    txParameters: TxParameters,
  ) => WithdrawLiquidityReturnType;
  swapCoins: (args: SwapCoinsParameters, txParameters: TxParameters) => SwapCoinsReturnType;
};

export function ammActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): AmmActions<transport, chain, signer> {
  return {
    // queries
    getPool: (args: GetPoolParameters) => getPool<chain, signer>(client, args),
    getAllPools: (args: GetAllPoolsParameters) => getAllPools<chain, signer>(client, args),
    getAmmConfig: (args: GetAmmConfigParameters) => getAmmConfig<chain, signer>(client, args),
    simulateSwap: (args: SimulateSwapParameters) => simulateSwap<chain, signer>(client, args),
    // mutations
    createPool: (args: CreatePoolParameters, txParameters: TxParameters) =>
      createPool<chain, signer>(client, args, txParameters),
    provideLiquidity: (args: ProvideLiquidityParameters, txParameters: TxParameters) =>
      provideLiquidity<chain, signer>(client, args, txParameters),
    withdrawLiquidity: (args: WithdrawLiquidityParameters, txParameters: TxParameters) =>
      withdrawLiquidity<chain, signer>(client, args, txParameters),
    swapCoins: (args: SwapCoinsParameters, txParameters: TxParameters) =>
      swapCoins<chain, signer>(client, args, txParameters),
  };
}
