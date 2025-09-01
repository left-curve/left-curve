import { tv } from "tailwind-variants";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import { View, Text } from "react-native";
import { twMerge } from "@left-curve/applets-kit";

export interface BadgeProps extends VariantProps<typeof badgeVariants> {
  text: string;
  classNames?: {
    base?: string;
    text?: string;
  };
}

export const Badge: React.FC<BadgeProps> = ({ text, classNames, ...rest }) => {
  const styles = badgeVariants({ ...rest });
  return (
    <View className={twMerge(styles.base(), classNames?.base)}>
      <Text className={twMerge(styles.text(), classNames?.text)}>{text}</Text>
    </View>
  );
};
const badgeVariants = tv(
  {
    slots: {
      base: "rounded-[4px] diatype-xs-medium w-fit h-fit",
      text: "",
    },
    variants: {
      color: {
        blue: {
          base: "bg-surface-secondary-blue border-tertiary-blue",
          text: "text-foreground-primary-blue",
        },
        red: {
          base: "bg-surface-secondary-red border-secondary-red",
          text: "text-foreground-primary-red",
        },
        green: {
          base: "bg-surface-tertiary-green border-surface-primary-green",
          text: "text-foreground-primary-green",
        },
      },
      size: {
        s: {
          base: "py-[2px] px-2",
          text: "diatype-xs-medium",
        },
        m: {
          base: "py-[3px] px-1 border",
          text: "diatype-xs-medium",
        },
      },
    },
    defaultVariants: {
      color: "blue",
      size: "m",
    },
  },
  {
    twMerge: true,
  },
);
