import { queryWasmSmart } from "@left-curve/sdk";

import type { Address, Chain, Client, Signer, Transport } from "@left-curve/sdk/types";
import type { Proposal, ProposalId } from "../../../types/safe.js";

export type SafeAccountGetProposalParameters = {
  address: Address;
  proposalId: ProposalId;
  height?: number;
};

export type SafeAccountGetProposalReturnType = Promise<Proposal>;

/**
 * Get the proposal by proposal ID
 * @param parameters
 * @param parameters.address The address of the account.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.height The height at which to query for the proposal.
 * @returns The proposal.
 */
export async function safeAccountGetProposal<
  chain extends Chain | undefined,
  signer extends Signer | undefined,
>(
  client: Client<Transport, chain, signer>,
  parameters: SafeAccountGetProposalParameters,
): SafeAccountGetProposalReturnType {
  const { proposalId, address, height = 0 } = parameters;
  const msg = {
    proposal: { proposalId },
  };

  return await queryWasmSmart(client, { contract: address, msg, height });
}
