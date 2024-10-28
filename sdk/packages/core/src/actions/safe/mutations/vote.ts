import type {
  Address,
  Chain,
  Client,
  ProposalId,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
  Vote,
} from "@leftcurve/types";
import { type ExecuteReturnType, execute } from "../../user/execute.js";

export type SafeAccountVoteParameters = {
  proposalId: ProposalId;
  sender: Address;
  account: Address;
  username: string;
  vote: Vote;
  execute: boolean;
};

export type SafeAccountVoteReturnType = ExecuteReturnType;

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
  const msg = { vote: voteMsg };

  const typedData: TypedDataParameter = {
    type: [{ name: "vote", type: "SafeVote" }],
    extraTypes: {
      SafeVote: [
        { name: "proposalId", type: "uint32" },
        { name: "username", type: "string" },
        { name: "vote", type: "string" },
        { name: "execute", type: "bool" },
      ],
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
