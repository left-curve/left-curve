import { type ReactNode } from "react";
import { View, Text, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type ChipVariant = "default" | "up" | "down" | "accent" | "outline";

export type ChipProps = ViewProps & {
  variant?: ChipVariant;
  children: ReactNode;
};

const variantStyles: Record<ChipVariant, string> = {
  default: "bg-bg-tint border-border-subtle",
  up: "bg-up-bg border-transparent",
  down: "bg-down-bg border-transparent",
  accent: "bg-accent-bg border-transparent",
  outline: "bg-transparent border-border-strong",
};

const variantTextStyles: Record<ChipVariant, string> = {
  default: "text-fg-primary",
  up: "text-up",
  down: "text-down",
  accent: "text-accent",
  outline: "text-fg-secondary",
};

export function Chip({ variant = "default", className, children, ...props }: ChipProps) {
  return (
    <View
      className={twMerge(
        "inline-flex items-center gap-1",
        "h-[22px] px-2",
        "rounded-chip border",
        "text-[11px] font-medium",
        "tracking-wide",
        variantStyles[variant],
        className,
      )}
      {...props}
    >
      {typeof children === "string" ? (
        <Text className={twMerge("text-[11px] font-medium", variantTextStyles[variant])}>
          {children}
        </Text>
      ) : (
        children
      )}
    </View>
  );
}
