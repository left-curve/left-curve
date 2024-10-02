import type React from "react";

interface Props {
  votes: {
    positive: number;
    negative: number;
  };
  totalWeight: number;
  threshold: number;
}

export const ProposalBar: React.FC<Props> = ({ threshold, votes, totalWeight }) => {
  const { positive, negative } = votes;
  return (
    <div className="relative w-full rounded-sm h-2 bg-surface-yellow-300">
      <div
        className="h-full bg-brand-green absolute top-0 left-0 rounded-sm z-20"
        style={{ width: `${(positive / totalWeight) * 100 + 1}%` }}
      />
      <div
        className="h-full bg-brand-pink absolute top-0 rounded-sm z-10"
        style={{
          width: `${(negative / totalWeight) * 100 + 1}%`,
          left: `${(positive / totalWeight) * 100 - 1}%`,
        }}
      />
      <div
        className="bg-black absolute top-1/2 h-4 w-[2px] rounded-lg transform -translate-y-1/2 z-30"
        style={{
          left: `${(threshold * 100) / totalWeight}%`,
        }}
      />
    </div>
  );
};
