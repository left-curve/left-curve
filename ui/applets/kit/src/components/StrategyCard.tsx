import { useConfig } from "@left-curve/store";
import { Button } from "#components/Button.js";

import type { PairSymbols, PairUpdate } from "@left-curve/dango/types";
import type React from "react";
import { twMerge } from "#utils/twMerge.js";

type StrategyCardProps = {
  pair: PairUpdate;
  onSelect: (pairSymbols: PairSymbols) => void;
  index?: number;
  labels: {
    party: string;
    earn: string;
    deposit: string;
    select: string;
    apy: string;
    tvl: string;
  };
};

const images = [
  {
    character: "/images/characters/cocodrile.svg",
    alt: "cocodrile",
    className: "w-[15.7rem] top-[7rem] right-4 ",
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
    className: "w-[23.5rem] top-[4rem] right-[-2rem]",
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

export const StrategyCard: React.FC<StrategyCardProps> = ({
  pair,
  onSelect,
  labels,
  index = 0,
}) => {
  const { baseDenom, quoteDenom } = pair;
  const { coins } = useConfig();

  const baseCoin = coins[baseDenom];
  const quoteCoin = coins[quoteDenom];

  return (
    <div className="relative p-4 min-h-[21.125rem] min-w-[17.375rem] bg-bg-tertiary-rice shadow-account-card rounded-xl overflow-hidden">
      <img
        src={images[index].character}
        alt={images[index].alt}
        className={twMerge(
          "absolute z-0 opacity-10 pointer-events-none select-none",
          images[index].className,
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
            <p className="diatype-lg-medium text-tertiary-500">
              {labels.deposit}{" "}
              <span className="font-bold">
                {baseCoin.symbol}-{quoteCoin.symbol}
              </span>
            </p>
            <p className="diatype-lg-medium text-tertiary-500">
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
              <span className="text-tertiary-500 diatype-xs-medium">{labels.apy}</span>
              <span className="text-secondary-700 diatype-sm-bold">-</span>
            </div>
            <div className="flex gap-2 items-center">
              <span className="text-tertiary-500 diatype-xs-medium">{labels.tvl}</span>
              <span className="text-secondary-700 diatype-sm-bold">-</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
