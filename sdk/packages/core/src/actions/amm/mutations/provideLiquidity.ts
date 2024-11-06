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
} from "@leftcurve/types";
import { getAppConfig } from "../../public/getAppConfig.js";
import { type ExecuteReturnType, execute } from "../../user/execute.js";

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

  const contract = await getAppConfig<Address>(client, { key: "amm" });

  return await execute(client, {
    sender,
    contract,
    msg,
    funds,
    gasLimit,
    typedData,
  });
}
