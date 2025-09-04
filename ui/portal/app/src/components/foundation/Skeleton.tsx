import { View } from "react-native";

import { twMerge } from "@left-curve/foundation";

import type React from "react";
import type { ViewProps } from "react-native";

type SkeletonProps = {
  className?: string;
} & ViewProps;

export const Skeleton: React.FC<SkeletonProps> = ({ className, ...rest }) => {
  return (
    <View className={twMerge("animate-pulse bg-rice-100/50 rounded-xs", className)} {...rest} />
  );
};
