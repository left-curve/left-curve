import type {
  Address,
  Chain,
  Client,
  Message,
  Signer,
  Transport,
  TxMessageType,
  TxParameters,
  TypedDataParameter,
} from "@leftcurve/types";
import { type ExecuteReturnType, execute } from "../../signer/execute.js";

export type SafeAccountProposeParameters = {
  sender: Address;
  account: Address;
  title: string;
  description?: string;
  messages: Message[];
  typedData?: TypedDataParameter<TxMessageType>;
};

export type SafeAccountProposeReturnType = ExecuteReturnType;

/**
 * Create a proposal in a safe account.
 * @param parameters
 * @param parameters.sender The sender of the proposal.
 * @param parameters.account The safe account address.
 * @param parameters.title The title of the proposal.
 * @param parameters.description The description of the proposal.
 * @param parameters.messages The messages to execute.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The tx hash of the transaction.
 */
export async function safeAccountPropose<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountProposeParameters,
  txParameters: TxParameters,
): SafeAccountProposeReturnType {
  const { sender, account, typedData: providedTypedData, ...proposeMsg } = parameters;
  const { gasLimit, funds } = txParameters;

  const msg = { propose: proposeMsg };

  const { extraTypes = {}, type = [] } = providedTypedData || {};

  const typedData: TypedDataParameter = {
    type: [{ name: "propose", type: "SafePropose" }],
    extraTypes: {
      SafePropose: [
        { name: "title", type: "string" },
        { name: "description", type: "string" },
        { name: "messages", type: "Message[]" },
      ],
      Message: type,
      ...extraTypes,
    },
  };

  return await execute(client, {
    sender,
    contract: account,
    msg,
    funds,
    gasLimit,
    typedData,
  });
}
