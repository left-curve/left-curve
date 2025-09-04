import { twMerge } from "@left-curve/foundation";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";

type PairAssetsProps = {
  assets: AnyCoin[];
  className?: string;
  mL?: (i: number) => string;
};

export const PairAssets: React.FC<PairAssetsProps> = ({ assets, mL, className }) => {
  return (
    <div className={"flex"}>
      {assets.map((asset, i) => (
        <img
          key={`asset-logo-${asset.symbol}-${i}`}
          src={asset.logoURI}
          alt={asset.symbol}
          className={twMerge("min-w-8 min-h-8 w-8 h-8 object-fit", className)}
          loading="lazy"
          style={{ marginLeft: mL ? mL(i) : `${-i}rem` }}
        />
      ))}
    </div>
  );
};
