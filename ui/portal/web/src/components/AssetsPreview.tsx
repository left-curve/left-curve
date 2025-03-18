import { Button } from "@left-curve/applets-kit";
import type { Coins } from "@left-curve/dango/types";
import { useChainId, useConfig, usePrices } from "@left-curve/store-react";
import type React from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";

interface Props {
  balances: Coins;
  showAllAssets?: () => void;
}

export const AssetsPreview: React.FC<Props> = ({ balances, showAllAssets }) => {
  const config = useConfig();
  const chainId = useChainId();
  const { formatNumberOptions } = useApp();

  const coins = config.coins[chainId];

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const sortedCoinsByBalance = Object.entries(coins).sort(([denomA], [denomB]) => {
    const balanceA = BigInt(balances[denomA] || "0");
    const balanceB = BigInt(balances[denomB] || "0");
    return balanceB > balanceA ? 1 : -1;
  });

  return (
    <div className="flex-col bg-rice-25 [box-shadow:0px_-1px_2px_0px_#F1DBBA80,_0px_2px_4px_0px_#AB9E8A66] rounded-md p-4 gap-4 w-full">
      <div className="flex items-center justify-between w-full">
        <p className="text-md font-bold">{m["common.assets"]()}</p>
        {showAllAssets ? (
          <Button variant="link" size="xs" onClick={showAllAssets}>
            {m["common.viewAll"]()}
          </Button>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-4 items-center justify-between">
        {sortedCoinsByBalance.map(([denom, coin]) => {
          return (
            <div className="flex gap-2 items-center" key={`preview-asset-${denom}`}>
              <img src={coin.logoURI} alt={coin.name} className="h-7 w-7 drag-none select-none" />
              <div className="flex flex-col text-xs">
                <p>{coin.symbol}</p>
                <p className="text-gray-500">
                  {getPrice(balances[denom] || "0", denom, { format: true })}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
