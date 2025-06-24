import { useConfig } from "@left-curve/store";
import { Button } from "#components/Button.js";

import type { PairSymbols, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import { twMerge } from "#utils/twMerge.js";

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

const PAIR_IMAGE_IDS: Record<string, number> = {
  "ETH-USDC": 0,
  "SOL-USDC": 1,
  "BTC-USDC": 2,
  "DGX-USDC": 3,
};

const images = [
  {
    character: "/images/characters/cocodrile.svg",
    alt: "cocodrile",
    className: "top-[7rem] right-4 w-[15,7rem]",
  },
  {
    character: "/images/characters/cobra.svg",
    alt: "Cobra",
    className: "w-[14rem] top-[7rem] right-[-3rem]",
  },
  {
    character: "/images/characters/bird.svg",
    alt: "Bird",
    className: "w-[19.56rem] top-[8rem] right-[-6rem]",
  },
  {
    character: "/images/characters/hippo.svg",
    alt: "Hippo",
    className: "w-[23.5rem] top-[6rem] left-0",
  },
  {
    character: "/images/characters/monkeys.svg",
    alt: "Monkeys",
    className: "w-[13rem] top-[4rem] right-[-2rem] scale-x-[-1]",
  },
  {
    character: "/images/characters/friends.svg",
    alt: "Friends",
    className: "w-[18.5rem] top-[5.5rem] right-[-5rem]",
  },
  {
    character: "/images/characters/green-octopus.svg",
    alt: "Green Octopus",
    className: "w-[9.5rem] top-[4rem] right-[-2rem]",
  },
];

function getPairImageId(baseSymbol: string, quoteSymbol: string): number {
  const key = `${baseSymbol}-${quoteSymbol}`;
  if (PAIR_IMAGE_IDS[key] !== undefined) return PAIR_IMAGE_IDS[key];
  // Fallback to a hash function if the pair is not predefined
  const hash = Array.from(key).reduce((acc, char) => acc + char.charCodeAt(0), 0);
  return hash % images.length;
}

export const StrategyCard: React.FC<StrategyCardProps> = ({ pair, onSelect, labels }) => {
  const { baseDenom, quoteDenom } = pair;
  const { coins } = useConfig();

  const baseCoin = coins[baseDenom];
  const quoteCoin = coins[quoteDenom];

  const imageId = getPairImageId(baseCoin.symbol, quoteCoin.symbol);
  const image = images[imageId];

  return (
    <div className="relative p-4  min-h-[21.125rem] min-w-[17.375rem] bg-rice-50 shadow-account-card rounded-xl overflow-hidden">
      <img
        src={image.character}
        alt={image.alt}
        className={twMerge(
          "absolute z-0 opacity-10 pointer-events-none select-none",
          image.className,
        )}
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
