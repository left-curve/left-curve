import type { AppConfig, TokenFactoryExecuteMsg } from "../../../types/index.js";
import { type ExecuteReturnType, execute } from "../../app/execute.js";

import { getAppConfig } from "@left-curve/sdk";
import type {
  Address,
  Chain,
  Client,
  Denom,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
} from "@left-curve/types";

export type BurnTokenParameters = {
  sender: Address;
  denom: Denom;
  from: Address;
  amount: string;
};

export type BurnTokenReturnType = ExecuteReturnType;

/**
 * Burn the token of the specified subdenom and amount
 * @param parameters
 * @param parameters.sender The sender of the pool creation.
 * @param parameters.denom The sub-denomination of the token.
 * @param parameters.from The recipient of the burn token.
 * @param parameters.amount The amount of the token to burn.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function burnToken<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: BurnTokenParameters,
  txParameters: TxParameters,
): BurnTokenReturnType {
  const { sender, denom, from, amount } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: TokenFactoryExecuteMsg = { burn: { denom, from, amount } };

  const typedData: TypedDataParameter = {
    type: [{ name: "BurnToken", type: "BurnToken" }],
    extraTypes: {
      BurnToken: [
        { name: "denom", type: "string" },
        { name: "amount", type: "string" },
        { name: "from", type: "address" },
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
