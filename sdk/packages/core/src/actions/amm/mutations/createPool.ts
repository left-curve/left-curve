import { getAppConfig } from "../../public/getAppConfig.js";
import { type ExecuteReturnType, execute } from "../../signer/execute.js";

import type {
  Address,
  AmmExecuteMsg,
  Chain,
  Client,
  PoolParams,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
  TypedDataProperty,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type CreatePoolParameters = {
  sender: Address;
  params: PoolParams;
};

export type CreatePoolReturnType = ExecuteReturnType;

/**
 * Creates a new trading pool.
 * @param parameters
 * @param parameters.sender The sender of the pool creation.
 * @param parameters.params The parameters of the pool to create.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function createPool<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: CreatePoolParameters,
  txParameters: TxParameters,
): CreatePoolReturnType {
  const { sender, params } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: AmmExecuteMsg = { createPool: params };

  const { type, extraTypes } = (() => {
    if ("xyk" in params) {
      return {
        type: [{ name: "xyk", type: "XykParams" }],
        extraTypes: {
          XykParams: [{ name: "liquidityFeeRate", type: "string" }],
        },
      };
    }

    if ("concentrated" in params) {
      return {
        type: [{ name: "concentrated", type: "ConcentratedParams" }],
        extraTypes: {
          ConcentratedParams: [],
        },
      };
    }

    throw new Error("Invalid pool type");
  })();

  const typedData: TypedDataParameter = {
    type: [{ name: "CreatePool", type: "CreatePool" }, ...type],
    extraTypes: {
      CreatePool: [{ name: "poolId", type: "uint32" }],
      ...(extraTypes as unknown as Record<string, TypedDataProperty[]>),
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
