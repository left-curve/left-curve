import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import type { AmmExecuteMsg, AppConfig, PoolId } from "../../../types/index.js";

import { getAppConfig } from "@left-curve/sdk";
import type {
  Address,
  Chain,
  Client,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
} from "@left-curve/types";

export type ProvideLiquidityParameters = {
  sender: Address;
  poolId: PoolId;
  minimumOutput?: string;
};

export type ProvideLiquidityReturnType = ExecuteReturnType;

/**
 * Provides liquidity to a trading pool.
 * @param parameters
 * @param parameters.sender The sender when providing liquidity.
 * @param parameters.poolId The pool ID to provide liquidity to.
 * @param parameters.minimumOutput The minimum output when providing liquidity.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function provideLiquidity<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: ProvideLiquidityParameters,
  txParameters: TxParameters,
): ProvideLiquidityReturnType {
  const { sender, poolId, minimumOutput } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: AmmExecuteMsg = { provideLiquidity: { poolId, minimumOutput } };

  const typedData: TypedDataParameter = {
    type: [{ name: "provideLiquidity", type: "ProvideLiquidity" }],
    extraTypes: {
      ProvideLiquidity: [
        { name: "poolId", type: "uint32" },
        { name: "minimumOutput", type: "string" },
      ],
    },
  };

  const { addresses } = await getAppConfig<AppConfig>(client);

  return await execute(client, {
    sender,
    contract: addresses.amm,
    msg,
    funds,
    gasLimit,
    typedData,
  });
}
