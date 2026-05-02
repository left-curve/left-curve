import { View, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

export type SkeletonProps = ViewProps & {
  width?: string | number;
  height?: string | number;
  rounded?: boolean;
};

export function Skeleton({
  width,
  height,
  rounded = false,
  className,
  style,
  ...props
}: SkeletonProps) {
  return (
    <View
      className={twMerge(
        "animate-pulse",
        "bg-bg-tint",
        rounded ? "rounded-full" : "rounded-field",
        className,
      )}
      style={{
        width: typeof width === "number" ? `${width}px` : width,
        height: typeof height === "number" ? `${height}px` : height,
        ...(typeof style === "object" ? style : {}),
      }}
      {...props}
    />
  );
}
