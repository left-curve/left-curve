import { type ExecuteReturnType, execute } from "../../app/execute.js";
import { getAppConfig } from "../../public/getAppConfig.js";

import type {
  Address,
  Chain,
  Client,
  Denom,
  Signer,
  TokenFactoryExecuteMsg,
  Transport,
  TxParameters,
  TypedDataParameter,
} from "@left-curve/types";
import type { DangoAppConfigResponse } from "@left-curve/types/dango";

export type MintTokenParameters = {
  sender: Address;
  denom: Denom;
  to: Address;
  amount: string;
};

export type MintTokenReturnType = ExecuteReturnType;

/**
 * Mint the token of the specified subdenom and amount to a recipient.
 * @param parameters
 * @param parameters.sender The sender of the pool creation.
 * @param parameters.denom The sub-denomination of the token.
 * @param parameters.to The recipient of the minted token.
 * @param parameters.amount The amount of the token to mint.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function mintToken<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: MintTokenParameters,
  txParameters: TxParameters,
): MintTokenReturnType {
  const { sender, denom, to, amount } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: TokenFactoryExecuteMsg = { mint: { denom, to, amount } };

  const typedData: TypedDataParameter = {
    type: [{ name: "MintToken", type: "MintToken" }],
    extraTypes: {
      MintToken: [
        { name: "denom", type: "string" },
        { name: "amount", type: "string" },
        { name: "to", type: "address" },
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
