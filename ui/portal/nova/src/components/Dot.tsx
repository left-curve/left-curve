import { View, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type DotVariant = "default" | "up" | "down" | "warn";

export type DotProps = ViewProps & {
  variant?: DotVariant;
  pulse?: boolean;
};

const variantStyles: Record<DotVariant, string> = {
  default: "bg-fg-tertiary",
  up: "bg-up",
  down: "bg-down",
  warn: "bg-warn",
};

export function Dot({ variant = "default", pulse = false, className, ...props }: DotProps) {
  return (
    <View
      className={twMerge(
        "inline-block shrink-0",
        "w-[7px] h-[7px] rounded-full",
        variantStyles[variant],
        pulse && "animate-pulse",
        className,
      )}
      {...props}
    />
  );
}
