import { usePublicClient } from "@left-curve/react";
import { safeAccountGetProposals } from "@left-curve/sdk/actions";
import { useQuery } from "@tanstack/react-query";

import { ProposalCard } from "./ProposalCard";

import type { Account } from "@left-curve/types";

interface Props {
  account: Account;
}

export const SafeProposalsTable: React.FC<Props> = ({ account }) => {
  const client = usePublicClient();

  const { data: proposals = {} } = useQuery({
    queryKey: ["account_proposals", account.address],
    queryFn: async () => safeAccountGetProposals(client, { address: account.address }),
  });

  return (
    <div className="flex flex-col gap-3 p-4 md:py-8 rounded-3xl w-full bg-surface-yellow-100">
      {Object.entries(proposals).map(([proposalId, proposal]) => (
        <ProposalCard
          key={proposal.title}
          accountAddr={account.address}
          proposal={proposal}
          proposalId={Number(proposalId)}
        />
      ))}
    </div>
  );
};
