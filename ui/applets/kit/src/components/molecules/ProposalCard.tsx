import { usePublicClient } from "@left-curve/store-react";
import { useQuery } from "@tanstack/react-query";
import { ProposalBar } from "../atoms/ProposalBar";

import type { Proposal } from "@left-curve/dango/types";
import type { Address } from "@left-curve/types";

interface Props {
  proposalId: number;
  proposal: Proposal;
  accountAddr: Address;
}

export const ProposalCard: React.FC<Props> = ({ proposalId, proposal, accountAddr }) => {
  const { title, status } = proposal;
  const client = usePublicClient();

  useQuery({
    queryKey: ["proposal_votes", proposalId],
    queryFn: async () => {},
  });

  return (
    <div className="p-4 md:p-6 rounded-2xl bg-surface-yellow-200 flex flex-col gap-4">
      <div className="flex gap-6 flex-col lg:flex-row ">
        <div className="grid grid-cols-[60px_1fr] gap-2 flex-1">
          <p className="font-bold">Title:</p>
          <p>{title}</p>
          <p className="font-bold">Status:</p>
          <p>{typeof status === "string" ? status : Object.keys(status).at(0)}</p>
        </div>
        <div className="flex flex-col">
          <p className="font-bold">Expired:</p>
          <p>{new Date().toDateString()}</p>
        </div>
      </div>
      <ProposalBar votes={{ positive: 1, negative: 2 }} threshold={1} totalWeight={4} />
    </div>
  );
};
