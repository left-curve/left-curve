import { queryWasmSmart } from "@left-curve/sdk";

import type { Address, Chain, Client, Signer, Transport } from "@left-curve/types";
import type { ProposalId, Username, Vote } from "../../../types/index.js";

export type SafeAccountGetVotesParameters = {
  address: Address;
  proposalId: ProposalId;
  height?: number;
};

export type SafeAccountGetVotesReturnType = Promise<Record<Username, Vote>>;

/**
 * Get the votes for a proposal.
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.height The height at which to query the votes for the proposal.
 * @returns The votes for the proposal.
 */
export async function safeAccountGetVotes<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountGetVotesParameters,
): SafeAccountGetVotesReturnType {
  const { proposalId, address, height = 0 } = parameters;
  const msg = {
    votes: { proposalId },
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}
