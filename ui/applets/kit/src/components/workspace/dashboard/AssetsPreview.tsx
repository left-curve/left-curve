import type { Coins } from "@left-curve/dango/types";
import { useChainId, useConfig, usePrices } from "@left-curve/store-react";
import type React from "react";
import { Button } from "../../foundation/Button";

interface Props {
  balances: Coins;
  showAllAssets?: () => void;
}

export const AssetsPreview: React.FC<Props> = ({ balances, showAllAssets }) => {
  const config = useConfig();
  const chainId = useChainId();

  const coins = config.coins[chainId];

  const { calculateBalance } = usePrices();

  return (
    <div className="hidden md:flex flex-col bg-rice-25 [box-shadow:0px_-1px_2px_0px_#F1DBBA80,_0px_2px_4px_0px_#AB9E8A66] rounded-md p-4 gap-4 w-full">
      <div className="flex items-center justify-between w-full">
        <p className="text-md font-bold">Assets</p>
        {showAllAssets ? (
          <Button variant="link" size="xs" onClick={showAllAssets}>
            View all
          </Button>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-4 items-center justify-between">
        {Object.entries(coins).map(([denom, coin]) => {
          return (
            <div className="flex gap-2 items-center" key={`preview-asset-${denom}`}>
              <img src={coin.logoURI} alt={coin.name} className="rounded-xl h-7 w-7" />
              <div className="flex flex-col text-xs">
                <p>{coin.symbol}</p>
                <p className="text-gray-500">
                  {calculateBalance({ [denom]: balances[denom] || "0" }, { format: true })}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
