import { CoinSelector, Skeleton, useControlledState } from "@left-curve/applets-kit";
import { useAppConfig, useConfig } from "@left-curve/store";
import type React from "react";

type PairAssetSelectorProps = {
  value: string;
  onChange: (denom: string) => void;
};

export const PairAssetSelector: React.FC<PairAssetSelectorProps> = ({ value, onChange }) => {
  const { coins } = useConfig();
  const { data: config } = useAppConfig();
  const pairCoins = Object.keys(config?.pairs || {});

  const coinPairs = Object.values(coins.byDenom).filter((c) => pairCoins.includes(c.denom));

  const [state, setState] = useControlledState<string>(value, onChange);

  return coinPairs.length ? (
    <CoinSelector
      coins={Object.values(coins.byDenom).filter(
        (c) => pairCoins.includes(c.denom) && c.denom !== "dango",
      )}
      value={state}
      onChange={(v) => setState(coins.byDenom[v].symbol)}
    />
  ) : (
    <Skeleton className="w-36 h-11" />
  );
};
