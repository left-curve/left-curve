import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";

type PairAssetsProps = {
  assets: AnyCoin[];
};

export const PairAssets: React.FC<PairAssetsProps> = ({ assets }) => {
  return (
    <div className={"flex"}>
      {assets.map((asset, i) => (
        <img
          key={`asset-logo-${asset.symbol}-${i}`}
          src={asset.logoURI}
          alt={asset.symbol}
          className="min-w-8 min-h-8 w-8 h-8 object-fit"
          loading="lazy"
          style={{ marginLeft: `${-i}rem` }}
        />
      ))}
    </div>
  );
};
