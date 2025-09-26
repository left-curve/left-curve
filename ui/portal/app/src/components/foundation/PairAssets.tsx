import type React from "react";
import { View, Image } from "react-native";
import { twMerge } from "@left-curve/foundation";
import type { AnyCoin } from "@left-curve/store/types";

type PairAssetsProps = {
  assets: AnyCoin[];
  className?: string;
};

export const PairAssets: React.FC<PairAssetsProps> = ({ assets, className }) => {
  return (
    <View className="flex flex-row">
      {assets.map((asset, i) => (
        <Image
          key={`asset-logo-${asset.symbol}-${i}`}
          source={{ uri: asset.logoURI }}
          accessibilityLabel={asset.symbol}
          className={twMerge("w-8 h-8", className)}
          style={{
            marginLeft: -i * 8,
          }}
        />
      ))}
    </View>
  );
};
