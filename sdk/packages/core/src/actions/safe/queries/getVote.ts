import type {
  Address,
  Chain,
  Client,
  ProposalId,
  Signer,
  Transport,
  Username,
  Vote,
} from "@leftcurve/types";
import { queryWasmSmart } from "../../public/queryWasmSmart.js";

export type SafeAccountGetVoteParameters = {
  address: Address;
  member: Username;
  proposalId: ProposalId;
  height?: number;
};

export type SafeAccountGetVoteReturnType = Promise<Vote | undefined>;

/**
 * Get the votes for a proposal.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.member The vote member username.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.height The height at which to query the votes for the proposal.
 * @returns The votes for the proposal.
 */
export async function safeAccountGetVote<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountGetVoteParameters,
): SafeAccountGetVoteReturnType {
  const { proposalId, member, address, height = 0 } = parameters;
  const msg = {
    votes: { proposalId, member },
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}
