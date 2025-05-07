import { createContext } from "@left-curve/applets-kit";
import { useSimpleSwap as state } from "@left-curve/store";

import { Badge } from "@left-curve/applets-kit";

import type { UseSimpleSwapParameters } from "@left-curve/store";
import type { PropsWithChildren } from "react";

const [SimpleSwapProvider, useSimpleSwap] = createContext<ReturnType<typeof state>>({
  name: "SimpleSwapContext",
});

const Root: React.FC<PropsWithChildren<UseSimpleSwapParameters>> = ({
  children,
  ...parameters
}) => {
  return <SimpleSwapProvider value={state(parameters)}>{children}</SimpleSwapProvider>;
};

const SimpleSwapHeader: React.FC = () => {
  const { statistics, quote } = useSimpleSwap();
  const { tvl, apy, volume } = statistics.data;
  return (
    <div className="flex flex-col gap-3 rounded-3xl bg-rice-50 shadow-card-shadow p-4 relative overflow-hidden mb-4">
      <div className="flex gap-2 items-center relative z-10">
        <img src={quote.coin.logoURI} alt="token" className="h-6 w-6" />
        <p className="text-gray-700 h4-bold">{quote.coin.symbol}</p>
        <Badge text="Stable Strategy" color="green" size="s" />
      </div>
      <div className="flex items-center justify-between gap-2 relative z-10 min-h-[22px]">
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">APY</p>
          <p className="text-gray-700 diatype-xs-bold">{apy}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">24h </p>
          <p className="text-gray-700 diatype-xs-bold">{volume}</p>
        </div>
        <div className="flex items-center gap-2">
          <p className="text-gray-500 diatype-xs-medium">TVL</p>
          <p className="text-gray-700 diatype-xs-bold">{tvl}</p>
        </div>
      </div>
      <img
        src="/images/characters/hippo.svg"
        alt=""
        className="absolute right-[-2.8rem] top-[-0.5rem] opacity-10"
      />
    </div>
  );
};

export const SimpleSwap = Object.assign(Root, {
  Header: SimpleSwapHeader,
});
