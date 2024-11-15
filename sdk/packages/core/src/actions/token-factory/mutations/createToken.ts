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
  Username,
} from "@leftcurve/types";
import { getAppConfig } from "../../public/getAppConfig.js";
import { type ExecuteReturnType, execute } from "../../signer/execute.js";

export type CreateTokenParameters = {
  sender: Address;
  subdenom: Denom;
  /** If provided, the denom will be formatted as:
   * > factory/{username}/{subdenom}
   * Otherwise, it will be formatted as:
   * > factory/{sender_address}/{subdenom}
   */
  username?: Username;
  /** If not provided, use the message sender's address. */
  admin?: Address;
};

export type CreateTokenReturnType = ExecuteReturnType;

/**
 * Creates a new token with the given sub-denomination, and appoints an admin
 * @param parameters
 * @param parameters.sender The sender of the pool creation.
 * @param parameters.subdenom The sub-denomination of the token.
 * @param parameters.username The username to associate with the token.
 * @param parameters.admin The admin of the token.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The result of the transaction.
 */
export async function createToken<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: CreateTokenParameters,
  txParameters: TxParameters,
): CreateTokenReturnType {
  const { sender, subdenom, admin, username } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg: TokenFactoryExecuteMsg = { create: { subdenom, username, admin } };

  const typedData: TypedDataParameter = {
    type: [{ name: "CreateToken", type: "CreateToken" }],
    extraTypes: {
      CreateToken: [
        { name: "subdenom", type: "string" },
        { name: "username", type: "string" },
        { name: "admin", type: "address" },
      ],
    },
  };

  const contract = await getAppConfig<Address>(client, { key: "token_factory" });

  return await execute(client, {
    sender,
    contract,
    msg,
    funds,
    gasLimit,
    typedData,
  });
}
