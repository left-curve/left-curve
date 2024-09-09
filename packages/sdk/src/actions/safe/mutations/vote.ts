import type {
  Address,
  Chain,
  Client,
  Hex,
  Json,
  Signer,
  Transport,
  TxParameters,
  Vote,
} from "@leftcurve/types";
import { execute } from "~/actions/user/execute";

export type SafeAccountVoteParameters = {
  sender: Address;
  account: Address;
  username: string;
  vote: Vote;
  execute: boolean;
};

export type SafeAccountVoteReturnType = Promise<Hex>;

/**
 * Vote on a proposal during its voting period.
 * @param parameters
 * @param parameters.sender The sender of the vote.
 * @param parameters.account The safe account address.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.vote The vote.
 * @param parameters.execute Whether to execute the proposal immediately.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The tx hash of the transaction.
 */
export async function safeAccountVote<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountVoteParameters,
  txParameters: TxParameters,
): SafeAccountVoteReturnType {
  const { sender, account, ...voteMsg } = parameters;
  const { gasLimit, funds } = txParameters;

  return await execute(client, {
    sender,
    contract: account,
    msg: voteMsg as Json,
    funds,
    gasLimit,
  });
}
