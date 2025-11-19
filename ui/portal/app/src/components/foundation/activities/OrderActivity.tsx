import type React from "react";
import { Pressable, View, type GestureResponderEvent } from "react-native";
import { twMerge } from "@left-curve/foundation";
import { OrderType, type OrderTypes } from "@left-curve/dango/types";
import { IconLimitOrder } from "../icons/IconLimitOrder";
import { IconMarketOrder } from "../icons/IconMarketOrder";

type OrderActivityProps = {
  kind: OrderTypes;
  onClick?: () => void;
  children?: React.ReactNode;
};

export const OrderActivity: React.FC<OrderActivityProps> = ({ kind, onClick, children }) => {
  const isLimit = kind === OrderType.Limit;
  const Icon = isLimit ? IconLimitOrder : IconMarketOrder;

  const handlePress = (e: GestureResponderEvent) => {
    onClick?.();
  };

  return (
    <Pressable
      onPress={handlePress}
      className="flex flex-row items-start gap-2 max-w-full w-full overflow-hidden"
      accessibilityRole="button"
    >
      <View
        className={twMerge(
          "flex items-center justify-center min-w-7 min-h-7 w-7 h-7 rounded-sm",
          isLimit ? "bg-fg-tertiary-blue" : "bg-surface-primary-green",
        )}
      >
        <Icon className={twMerge(isLimit ? "text-ink-secondary-blue" : "text-brand-green")} />
      </View>

      <View className="flex flex-col max-w-[100%] overflow-hidden">{children}</View>
    </Pressable>
  );
};
