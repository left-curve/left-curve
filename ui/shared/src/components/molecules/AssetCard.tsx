import { useConfig, usePrices } from "@left-curve/react";
import { formatNumber, formatUnits } from "@left-curve/utils";

import type { Coin } from "@left-curve/types";

interface Props {
  coin: Coin;
}

export const AssetCard: React.FC<Props> = ({ coin }) => {
  const { coins, state } = useConfig();
  const coinInfo = coins[state.chainId][coin.denom];
  const humanAmount = formatUnits(coin.amount, coinInfo.decimals);

  const { getPrice } = usePrices();
  const price = getPrice(humanAmount, coin.denom, { format: true });

  return (
    <div className="bg-white px-4 py-2 rounded-3xl grid grid-cols-[1fr_100px_100px] items-center border border-white/50">
      <div className="flex gap-2 items-center">
        {coinInfo.logoURI ? (
          <img src={coinInfo.logoURI} className="h-8 w-8 rounded-full" alt={coinInfo.denom} />
        ) : (
          <div className="h-8 w-8 rounded-full bg-gray-200" />
        )}
        <div className="flex md:gap-2 text-sm flex-col md:flex-row">
          <p className="text-gray-400">{coinInfo.symbol}</p>
          <p>{coinInfo.name}</p>
        </div>
      </div>
      <div className="min-w-[3rem]">
        {formatNumber(humanAmount, { language: navigator.language })}
      </div>
      <div className="min-w-[3rem] text-end">{price}</div>
    </div>
  );
};
