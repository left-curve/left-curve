import { useConfig, usePrices } from "@leftcurve/react";
import type { Coin } from "@leftcurve/types";
import { twMerge } from "~/utils";

interface Props {
  deposited?: Coin;
  borrowed: Coin;
}

export const BorrowAssetCard: React.FC<Props> = ({ deposited, borrowed }) => {
  const { coins, state } = useConfig();
  const coinInfo = coins[state.chainId][borrowed.denom];

  const { getPrice } = usePrices();
  const borrowingAmountPrice = getPrice(borrowed.amount, borrowed.denom, { format: true });
  const coinAmountPrice = deposited?.amount
    ? getPrice(deposited.amount, deposited.denom, { format: true })
    : 0;

  return (
    <div
      className={twMerge(
        "bg-white p-4 rounded-3xl grid items-center",
        deposited ? "grid-cols-[1fr_100px_100px]" : "grid-cols-[1fr_100px]",
      )}
    >
      <div className="flex gap-2 items-center">
        <img src={coinInfo.logoURI} className="h-8 w-8 rounded-full" alt="usdc" />
        <div className="flex md:gap-2 text-sm flex-col md:flex-row">
          <p className="text-gray-400">{coinInfo.symbol}</p>
          <p>{coinInfo.name}</p>
        </div>
      </div>
      {deposited ? (
        <div className="min-w-[3rem] flex flex-col items-end">
          <p className="font-bold text-typography-black-300">{deposited.amount}</p>
          <p>{coinAmountPrice}</p>
        </div>
      ) : null}
      <div className="min-w-[3rem] flex flex-col items-end">
        <p className="font-bold text-typography-black-300">{borrowed.amount}</p>
        <p>{borrowingAmountPrice}</p>
      </div>
    </div>
  );
};
