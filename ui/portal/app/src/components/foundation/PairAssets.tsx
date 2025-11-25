import React from "react";
import { View } from "react-native";
import { twMerge } from "@left-curve/foundation";
import type { AnyCoin } from "@left-curve/store/types";
import { CoinIcon } from "./CoinIcon";

type PairAssetsProps = {
  assets: AnyCoin[];
  className?: string;
  size?: number;
};

export const PairAssets: React.FC<PairAssetsProps> = ({ assets, className, size = 20 }) => {
  return (
    <View className="flex flex-row items-center">
      {assets.map((asset, i) => (
        <View
          key={`asset-logo-${asset.symbol}-${i}`}
          style={{
            marginLeft: i === 0 ? 0 : -size * 0.4,
            width: size,
            height: size,
          }}
          className={twMerge("rounded-full overflow-hidden", className)}
        >
          <CoinIcon symbol={asset.symbol} size={size} />
        </View>
      ))}
    </View>
  );
};
