import { View, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

type SpinnerSize = "sm" | "default" | "lg";

export type SpinnerProps = ViewProps & {
  size?: SpinnerSize;
};

const sizeStyles: Record<SpinnerSize, string> = {
  sm: "w-3 h-3 border-[1.5px]",
  default: "w-4 h-4 border-2",
  lg: "w-6 h-6 border-2",
};

export function Spinner({ size = "default", className, ...props }: SpinnerProps) {
  return (
    <View
      role="status"
      aria-label="Loading"
      className={twMerge(
        "inline-block",
        "border-current border-r-transparent",
        "rounded-full",
        "animate-spin",
        "opacity-80",
        sizeStyles[size],
        className,
      )}
      {...props}
    />
  );
}
