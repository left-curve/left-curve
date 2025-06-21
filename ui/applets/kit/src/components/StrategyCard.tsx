import { useConfig } from "@left-curve/store";
import { Button } from "#components/Button.js";

import type { PairSymbols, PairUpdate } from "@left-curve/dango/types";
import type React from "react";

type StrategyCardProps = {
  pair: PairUpdate;
  onSelect: (pairSymbols: PairSymbols) => void;
  labels: {
    party: string;
    earn: string;
    deposit: string;
    select: string;
    apy: string;
    tvl: string;
  };
};

export const StrategyCard: React.FC<StrategyCardProps> = ({ pair, onSelect, labels }) => {
  const { baseDenom, quoteDenom } = pair;
  const { coins } = useConfig();

  const baseCoin = coins[baseDenom];
  const quoteCoin = coins[quoteDenom];

  return (
    <div className="relative p-4  min-h-[21.125rem] min-w-[17.375rem] bg-rice-50 shadow-account-card rounded-xl overflow-hidden">
      <img
        src="/images/characters/cocodrile.svg"
        alt=""
        className="absolute z-0 top-[7rem] right-4 w-[15,7rem] opacity-10 pointer-events-none select-none"
      />
      <div className="flex flex-col gap-2 justify-between z-10 w-full h-full relative">
        <div className="flex flex-col gap-6 items-center justify-center text-center">
          <div className="flex">
            <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="h-12 w-12" />
            <img src={quoteCoin.logoURI} alt={quoteCoin.symbol} className="h-12 w-12 -ml-6" />
          </div>
          <div className="flex flex-col gap-1">
            <p className="exposure-h3-italic">{`${baseCoin.symbol} ${labels.party}!`}</p>
            <p className="diatype-lg-medium text-gray-500">
              {labels.deposit}{" "}
              <span className="font-bold">
                {baseCoin.symbol}-{quoteCoin.symbol}
              </span>
            </p>
            <p className="diatype-lg-medium text-gray-500">
              {labels.earn} {quoteCoin.symbol}
            </p>
          </div>
        </div>
        <div className="flex flex-col gap-4">
          <Button
            size="lg"
            variant="secondary"
            fullWidth
            onClick={() => onSelect({ baseSymbol: baseCoin.symbol, quoteSymbol: quoteCoin.symbol })}
          >
            {labels.select}
          </Button>
          <div className="p-2 rounded-xl bg-rice-100/80 flex items-center justify-between">
            <div className="flex gap-2 items-center">
              <span className="text-gray-500 diatype-xs-medium">{labels.apy}</span>
              <span className="text-gray-700 diatype-sm-bold">-</span>
            </div>
            <div className="flex gap-2 items-center">
              <span className="text-gray-500 diatype-xs-medium">{labels.tvl}</span>
              <span className="text-gray-700 diatype-sm-bold">-</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
