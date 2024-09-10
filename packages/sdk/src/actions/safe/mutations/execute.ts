import type {
  Address,
  Chain,
  Client,
  Hex,
  ProposalId,
  Signer,
  Transport,
  TxParameters,
} from "@leftcurve/types";
import { execute } from "~/actions/user/execute";

export type SafeAccountExecuteParameters = {
  sender: Address;
  account: Address;
  proposalId: ProposalId;
};

export type SafeAccountExecuteReturnType = Promise<Hex>;

/**
 *  Execute a proposal once it's passed and the timelock (if there is one)
 * has elapsed.
 * @param parameters
 * @param parameters.sender The sender of the vote.
 * @param parameters.account The safe account address.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.execute Whether to execute the proposal immediately.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The tx hash of the transaction.
 */
export async function safeAccountExecute<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountExecuteParameters,
  txParameters: TxParameters,
): SafeAccountExecuteReturnType {
  const { sender, account, ...executeMsg } = parameters;
  const { gasLimit, funds } = txParameters;

  return await execute(client, {
    sender,
    contract: account,
    msg: executeMsg,
    funds,
    gasLimit,
  });
}
