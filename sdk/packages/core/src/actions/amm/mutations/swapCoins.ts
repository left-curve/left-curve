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
} from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export type SwapCoinsParameters = {
  sender: Address;
  route: PoolId[];
  minimumOutput?: string;
};

export type SwapCoinsReturnType = ExecuteReturnType;

/**
 * Executes a swap between two coins.
 * @param parameters
 * @param parameters.sender The sender of the swap.
 * @param parameters.route The route of the swap.
 * @param parameters.minimumOutput The minimum output of the swap.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function swapCoins<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SwapCoinsParameters,
  txParameters: TxParameters,
): SwapCoinsReturnType {
  const { sender, route, minimumOutput } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: AmmExecuteMsg = { swap: { route, minimumOutput } };

  const typedData: TypedDataParameter = {
    type: [{ name: "swap", type: "Swap" }],
    extraTypes: {
      Swap: [
        { name: "route", type: "uint128[]" },
        { name: "minimumOutput", type: "string" },
      ],
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
