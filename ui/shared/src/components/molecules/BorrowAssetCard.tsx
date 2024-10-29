import { useConfig, usePrices } from "@leftcurve/react";
import type { Coin } from "@leftcurve/types";
import { formatUnits } from "@leftcurve/utils";
import { twMerge } from "../../utils";

interface Props {
  deposited: Coin;
  borrowed: Coin;
}

export const BorrowAssetCard: React.FC<Props> = ({ deposited, borrowed }) => {
  const { coins, state } = useConfig();
  const { getPrice } = usePrices();

  const coinInfoBorrowed = coins[state.chainId][borrowed.denom];
  const humanBorrowedAmount = formatUnits(borrowed.amount, coinInfoBorrowed.decimals);
  const borrowingAmountPrice = getPrice(humanBorrowedAmount, borrowed.denom, { format: true });

  const coinInfoDeposited = coins[state.chainId][deposited.denom];
  const humanDepositedAmount = formatUnits(deposited.amount, coinInfoDeposited.decimals);
  const depositedAmountPrice = getPrice(humanDepositedAmount, deposited.denom, { format: true });

  return (
    <div
      className={twMerge(
        "bg-white p-4 rounded-3xl grid items-center",
        deposited ? "grid-cols-[1fr_100px_100px]" : "grid-cols-[1fr_100px]",
      )}
    >
      <div className="flex gap-2 items-center">
        <img src={coinInfoBorrowed.logoURI} className="h-8 w-8 rounded-full" alt="usdc" />
        <div className="flex md:gap-2 text-sm flex-col md:flex-row">
          <p className="text-gray-400">{coinInfoBorrowed.symbol}</p>
          <p>{coinInfoBorrowed.name}</p>
        </div>
      </div>
      {deposited ? (
        <div className="min-w-[3rem] flex flex-col items-end">
          <p className="font-bold text-typography-black-300">{humanDepositedAmount}</p>
          <p>{depositedAmountPrice}</p>
        </div>
      ) : null}
      <div className="min-w-[3rem] flex flex-col items-end">
        <p className="font-bold text-typography-black-300">{humanBorrowedAmount}</p>
        <p>{borrowingAmountPrice}</p>
      </div>
    </div>
  );
};
