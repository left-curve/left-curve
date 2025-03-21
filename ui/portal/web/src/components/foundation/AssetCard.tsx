import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices } from "@left-curve/store";

import type { Coin } from "@left-curve/dango/types";
import { useApp } from "~/hooks/useApp";

interface Props {
  coin: Coin;
}

export const AssetCard: React.FC<Props> = ({ coin }) => {
  const { coins, state } = useConfig();
  const { formatNumberOptions } = useApp();
  const coinInfo = coins[state.chainId][coin.denom];
  const humanAmount = formatUnits(coin.amount, coinInfo.decimals);

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const price = getPrice(humanAmount, coin.denom, { format: true });

  return (
    <div className="flex items-center justify-between p-4 hover:bg-rice-50">
      <div className="flex gap-2 items-center">
        {coinInfo.logoURI ? (
          <img src={coinInfo.logoURI} className="h-8 w-8 rounded-full" alt={coinInfo.denom} />
        ) : (
          <div className="h-8 w-8 rounded-full bg-gray-200" />
        )}
        <div className="flex flex-col text-base">
          <p className="text-gray-500">{coinInfo.symbol}</p>
          <p>{formatNumber(humanAmount, formatNumberOptions)}</p>
        </div>
      </div>
      <div className="flex flex-col">
        {/* <p className="text-gray-500">2,09%</p> */}
        <p>{price}</p>
      </div>
    </div>
  );
};
