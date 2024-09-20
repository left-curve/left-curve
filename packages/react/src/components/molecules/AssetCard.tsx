import { formatNumber } from "@leftcurve/utils";
import { useConfig, usePrices } from "~/hooks";

import type { Coin, Language } from "@leftcurve/types";

interface Props {
  coin: Coin;
}

export const AssetCard: React.FC<Props> = ({ coin }) => {
  const { coins, state } = useConfig();
  const coinInfo = coins[state.chainId][coin.denom];

  const language = navigator.language as Language;

  const { getPrice } = usePrices();
  const price = getPrice(coin.amount, coin.denom, { format: true });

  return (
    <div className="bg-white p-2 rounded-3xl grid grid-cols-[1fr_100px_100px] items-center">
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
      <div className="min-w-[3rem]">{formatNumber(coin.amount, { language })}</div>
      <div className="min-w-[3rem]">{price}</div>
    </div>
  );
};
