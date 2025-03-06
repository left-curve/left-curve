import type React from "react";
import { twMerge } from "../../utils";

interface Props {
  total: number;
  borrow: number;
  borrowed: number;
}

export const BorrowBar: React.FC<Props> = ({ borrow, borrowed, total }) => {
  const borrowPercentage = (borrow / total) * 100;
  const borrowedPercentage = (borrowed / total) * 100;
  console.log(borrowPercentage);
  console.log(borrowedPercentage);

  return (
    <div className="w-full flex flex-col gap-1">
      <div className="flex items-center justify-between">
        <span className="diatype-sm-medium text-gray-500">Borrow Capacity</span>
        <span className="diatype-sm-bold">${total}K</span>
      </div>
      <div className="border border-green-bean-500 rounded-full overflow-hidden bg-gray-400 relative z-10 h-3">
        <span
          className="absolute top-0 left-0 z-[11] h-full bg-borrow-bar-green"
          style={{ width: `${borrowPercentage.toFixed(2)}%` }}
        />
        <span
          className="absolute top-0 left-0 z-[12] h-full bg-borrow-bar-red"
          style={{ width: `${borrowedPercentage.toFixed(2)}%` }}
        />
      </div>
      <span className="diatype-sm-bold">
        ${borrowed.toFixed(2)}K of ${borrow.toFixed(2)}K
      </span>
    </div>
  );
};
