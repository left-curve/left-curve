import { View, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

export type DividerProps = ViewProps;

export function Divider({ className, ...props }: DividerProps) {
  return <View className={twMerge("h-px", "bg-border-subtle", className)} {...props} />;
}
