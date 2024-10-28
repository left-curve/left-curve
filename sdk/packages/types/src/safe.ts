import type { Username } from "./account.js";
import type { Duration, Timestamp } from "./common.js";
import type { Message } from "./tx.js";

/**
 * The number of votes a member has.
 * E.g. if a user has a power of 2, then each vote this member casts is counted
 * as two votes.
 */
export type Power = number;

/** The ID of a proposal. */
export type ProposalId = number;

export type Proposal = {
  title: string;
  description?: string;
  messages: Message[];
  status: ProposalStatus;
};

export type Safe = {
  /** Users who can votes in this Safe, and their respective voting power. */
  members: Record<Username, Power>;
  /** The period of time since a proposal's creation when votes can be casted. */
  votingPeriod: number;
  /** The minimum number of YES votes a proposal must receive in order to pass.
   * Must be between 1 and the total power across all members (inclusive).
   */
  threshold: Power;
  /** The minimum delay after a proposal is passed before it can be executed. */
  timelock?: Duration;
};

export type ProposalStatus = VotingStatus | PassedStatus | "failed" | "executed";

/** The proposal is being voted on by members. */
export type VotingStatus = {
  voting: {
    /** The time at which the voting period started.
     * Votes can be casted until this time plus the voting period.
     */
    since: Timestamp;
    /** Parameters for tallying the vote.
     *
     * These parameters can change at any time, so we save the parameters
     * _at the time the proposal was created_ inside the proposal.
     */
    params: Safe;
  };
};

/**
 * The proposal has received equal or more YES votes than the Safe's threshold,
 * and can be executed once the timelock (if any) is passed.
 */
export type PassedStatus = {
  passed: { execute_after: Timestamp };
};

/** The proposal has failed to receive a sufficient number of YES votes during
 * its voting period.
 */
export type FailedStatus = "failed";

/** The proposal has passed and been executed. */
export type ExecutedStatus = "executed";

/**
 * A vote to a proposal.
 * We currently don't support "abstain" or "no with veto" votes. If you need
 * them, please let us know.
 */
export const Vote = {
  Yes: "Yes",
  No: "No",
} as const;

export type Vote = (typeof Vote)[keyof typeof Vote];
