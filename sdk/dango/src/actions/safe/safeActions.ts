import type { Chain, Client, Transport, TxParameters } from "@left-curve/types";

import {
  type SafeAccountGetProposalParameters,
  type SafeAccountGetProposalReturnType,
  safeAccountGetProposal,
} from "./queries/getProposal.js";

import {
  type SafeAccountGetProposalsParameters,
  type SafeAccountGetProposalsReturnType,
  safeAccountGetProposals,
} from "./queries/getProposals.js";

import {
  type SafeAccountGetVoteParameters,
  type SafeAccountGetVoteReturnType,
  safeAccountGetVote,
} from "./queries/getVote.js";

import {
  type SafeAccountGetVotesParameters,
  type SafeAccountGetVotesReturnType,
  safeAccountGetVotes,
} from "./queries/getVotes.js";

import {
  type SafeAccountExecuteParameters,
  type SafeAccountExecuteReturnType,
  safeAccountExecute,
} from "./mutations/execute.js";

import {
  type SafeAccountProposeParameters,
  type SafeAccountProposeReturnType,
  safeAccountPropose,
} from "./mutations/propose.js";

import {
  type SafeAccountVoteParameters,
  type SafeAccountVoteReturnType,
  safeAccountVote,
} from "./mutations/vote.js";

import { DangoClient } from "../../types/clients.js";
import { Signer } from '../../types/signer.js';

export type SafeQueryActions = {
  safeAccountGetProposal: (
    args: SafeAccountGetProposalParameters,
  ) => SafeAccountGetProposalReturnType;
  safeAccountGetProposals: (
    args: SafeAccountGetProposalsParameters,
  ) => SafeAccountGetProposalsReturnType;
  safeAccountGetVote: (args: SafeAccountGetVoteParameters) => SafeAccountGetVoteReturnType;
  safeAccountGetVotes: (args: SafeAccountGetVotesParameters) => SafeAccountGetVotesReturnType;
};

export type SafeMutationActions = {
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

export function safeQueryActions<transport extends Transport = Transport>(
  client: Client<transport, Chain, Signer>,
): SafeQueryActions {
  return {
    safeAccountGetProposal: (args: SafeAccountGetProposalParameters) =>
      safeAccountGetProposal(client, args),
    safeAccountGetProposals: (args: SafeAccountGetProposalsParameters) =>
      safeAccountGetProposals(client, args),
    safeAccountGetVote: (args: SafeAccountGetVoteParameters) => safeAccountGetVote(client, args),
    safeAccountGetVotes: (args: SafeAccountGetVotesParameters) => safeAccountGetVotes(client, args),
  };
}

export function safeMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): SafeMutationActions {
  return {
    safeAccountExecute: (...args) => safeAccountExecute(client, ...args),
    safeAccountPropose: (...args) => safeAccountPropose(client, ...args),
    safeAccountVote: (...args) => safeAccountVote(client, ...args),
  };
}
