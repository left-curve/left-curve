import type {
  Address,
  Chain,
  Client,
  Hex,
  Message,
  Signer,
  Transport,
  TxParameters,
} from "@leftcurve/types";
import { execute } from "~/actions/user/execute";

export type SafeAccountProposeParameters = {
  sender: Address;
  account: Address;
  title: string;
  description?: string;
  messages: Message[];
};

export type SafeAccountProposeReturnType = Promise<Hex>;

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
  const { sender, account, ...proposeMsg } = parameters;
  const { gasLimit, funds } = txParameters;

  return await execute(client, {
    sender,
    contract: account,
    msg: proposeMsg,
    funds,
    gasLimit,
  });
}
