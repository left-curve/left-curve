import type { Address, Funds, Transport, TxParameters } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import type { DangoClient } from "../../../types/clients.js";
import type { ProposalId, Vote } from "../../../types/safe.js";
import type { Signer } from "../../../types/signer.js";
import type { TypedDataParameter } from "../../../types/typedData.js";

export type SafeAccountVoteParameters = {
  proposalId: ProposalId;
  sender: Address;
  account: Address;
  username: string;
  vote: Vote;
  funds?: Funds;
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
export async function safeAccountVote<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SafeAccountVoteParameters,
  txParameters: TxParameters,
): SafeAccountVoteReturnType {
  const { sender, account, funds = {}, ...voteMsg } = parameters;
  const { gasLimit } = txParameters;
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
    execute: {
      contract: account,
      msg,
      funds,
      typedData,
    },
    gasLimit,
  });
}
