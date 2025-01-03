import { getAppConfig } from "../../public/getAppConfig.js";
import { type ExecuteReturnType, execute } from "../../signer/execute.js";

import type {
  Address,
  AmmExecuteMsg,
  Chain,
  Client,
  PoolId,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type WithdrawLiquidityParameters = {
  sender: Address;
  poolId: PoolId;
};

export type WithdrawLiquidityReturnType = ExecuteReturnType;

/**
 * Withdraws liquidity from a trading pool.
 * @param parameters
 * @param parameters.sender The sender when withdrawing liquidity.
 * @param parameters.poolId The pool ID to withdraw liquidity from.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function withdrawLiquidity<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: WithdrawLiquidityParameters,
  txParameters: TxParameters,
): WithdrawLiquidityReturnType {
  const { sender, poolId } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: AmmExecuteMsg = { withdrawLiquidity: { poolId } };

  const typedData: TypedDataParameter = {
    type: [{ name: "withdrawLiquidity", type: "WithdrawLiquidity" }],
    extraTypes: {
      WithdrawLiquidity: [{ name: "poolId", type: "uint32" }],
    },
  };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  return await execute(client, {
    sender,
    contract: addresses.amm,
    msg,
    funds,
    gasLimit,
    typedData,
  });
}
