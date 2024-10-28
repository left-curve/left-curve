import type { Chain, Client, Signer, Transport, TxParameters } from "@leftcurve/types";

import {
  type SafeAccountGetProposalParameters,
  type SafeAccountGetProposalReturnType,
  safeAccountGetProposal,
} from "./queries/getProposal";

import {
  type SafeAccountGetProposalsParameters,
  type SafeAccountGetProposalsReturnType,
  safeAccountGetProposals,
} from "./queries/getProposals";

import {
  type SafeAccountGetVoteParameters,
  type SafeAccountGetVoteReturnType,
  safeAccountGetVote,
} from "./queries/getVote";

import {
  type SafeAccountGetVotesParameters,
  type SafeAccountGetVotesReturnType,
  safeAccountGetVotes,
} from "./queries/getVotes";

import {
  type SafeAccountExecuteParameters,
  type SafeAccountExecuteReturnType,
  safeAccountExecute,
} from "./mutations/execute";

import {
  type SafeAccountProposeParameters,
  type SafeAccountProposeReturnType,
  safeAccountPropose,
} from "./mutations/propose";

import {
  type SafeAccountVoteParameters,
  type SafeAccountVoteReturnType,
  safeAccountVote,
} from "./mutations/vote";

export type SafeActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  // queries
  safeAccountGetProposal: (
    args: SafeAccountGetProposalParameters,
  ) => SafeAccountGetProposalReturnType;
  safeAccountGetProposals: (
    args: SafeAccountGetProposalsParameters,
  ) => SafeAccountGetProposalsReturnType;
  safeAccountGetVote: (args: SafeAccountGetVoteParameters) => SafeAccountGetVoteReturnType;
  safeAccountGetVotes: (args: SafeAccountGetVotesParameters) => SafeAccountGetVotesReturnType;
  // mutations
  safeAccountExecute: (
    args: SafeAccountExecuteParameters,
    txArgs: TxParameters,
  ) => SafeAccountExecuteReturnType;
  safeAccountPropose: (
    args: SafeAccountProposeParameters,
    txArgs: TxParameters,
  ) => SafeAccountProposeReturnType;
  safeAccountVote: (
    args: SafeAccountVoteParameters,
    txArgs: TxParameters,
  ) => SafeAccountVoteReturnType;
};

export function safeActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): SafeActions<transport, chain, signer> {
  return {
    // queries
    safeAccountGetProposal: (args: SafeAccountGetProposalParameters) =>
      safeAccountGetProposal<chain, signer>(client, args),
    safeAccountGetProposals: (args: SafeAccountGetProposalsParameters) =>
      safeAccountGetProposals<chain, signer>(client, args),
    safeAccountGetVote: (args: SafeAccountGetVoteParameters) =>
      safeAccountGetVote<chain, signer>(client, args),
    safeAccountGetVotes: (args: SafeAccountGetVotesParameters) =>
      safeAccountGetVotes<chain, signer>(client, args),
    // mutations
    safeAccountExecute: (...args) => safeAccountExecute<chain, signer>(client, ...args),
    safeAccountPropose: (...args) => safeAccountPropose<chain, signer>(client, ...args),
    safeAccountVote: (...args) => safeAccountVote<chain, signer>(client, ...args),
  };
}
