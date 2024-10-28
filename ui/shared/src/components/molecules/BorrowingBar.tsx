import type { Language } from "@leftcurve/types";
import { formatNumber } from "@leftcurve/utils";
import type React from "react";
import { twMerge } from "../../utils";

interface Props {
  total: number;
  current: number;
  threshold: number;
}

export const BorrowingBar: React.FC<Props> = ({ total, current, threshold }) => {
  const currentPercentage = (current / total) * 100;
  const thresholdPercentage = (threshold / total) * 100;
  const language = navigator.language as Language;

  return (
    <div className="text-xs">
      <p className="uppercase text-typography-black-200 mb-2">Borrowing capacity</p>
      <div className="relative  rounded-xl h-4 w-full bg-typography-black-200">
        <div
          className={twMerge(
            "h-full absolute top-0 left-0 rounded-2xl z-10 bg-gradient-to-r from-gradient-start to-gradient-end bg-[length:270px_100%]",
          )}
          style={{ width: `${currentPercentage}%` }}
        />
        <div
          className="absolute top-[calc(100%-16px)] transform -translate-y-1/2 z-20 flex items-center justify-center flex-col w-8 -translate-x-4"
          style={{ left: `${thresholdPercentage}%` }}
        >
          <p className="text-typography-black-200">{formatNumber(threshold, { language })}</p>
          <div className="bg-typography-black-200 h-[1.2rem] w-[2px] rounded-lg" />
        </div>
      </div>
      <div>
        {formatNumber(current, { language })} of {formatNumber(total, { language })}
      </div>
    </div>
  );
};
