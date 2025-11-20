import { View } from "react-native";
import { COINS } from "~/constants";
import { GlobalText } from "./GlobalText";

import type React from "react";
import { twMerge } from "@left-curve/foundation";

type CoinIconProps = {
  symbol: string;
  size?: number;
  className?: string;
};

export const CoinIcon: React.FC<CoinIconProps> = ({ symbol, size = 20, className }) => {
  const Icon = COINS[symbol as keyof typeof COINS]?.default;

  if (!Icon) {
    return (
      <View
        style={{
          width: size,
          height: size,
        }}
        className={twMerge(
          "flex items-center justify-center bg-utility-gray-100 rounded-full w-5 h-5",
          className,
        )}
      >
        <View>
          <GlobalText style={{ color: "white", fontSize: size * 0.5 }}>{symbol[0]}</GlobalText>
        </View>
      </View>
    );
  }

  return <Icon width={size} height={size} className={className} />;
};
