import type { ReactNode } from "react";
import { View, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type CardVariant = "default" | "elevated" | "sunken";

export type CardProps = ViewProps & {
  variant?: CardVariant;
  children: ReactNode;
};

const variantStyles: Record<CardVariant, string> = {
  default: "bg-bg-surface border-border-subtle",
  elevated: "bg-bg-elev border-border-subtle shadow-md",
  sunken: "bg-bg-sunk border-border-subtle",
};

export function Card({ variant = "default", className, children, ...props }: CardProps) {
  return (
    <View className={twMerge("border rounded-card", variantStyles[variant], className)} {...props}>
      {children}
    </View>
  );
}
