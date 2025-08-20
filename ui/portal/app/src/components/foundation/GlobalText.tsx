import { Text, TextProps } from "react-native";
import { twMerge } from "@left-curve/applets-kit";

//TODO: Add typography (diatype & exposure)

export function GlobalText({ className, ...props }: TextProps & { className?: string }) {
  return <Text className={twMerge("text-primary-900 diatype-m-medium", className)} {...props} />;
}
