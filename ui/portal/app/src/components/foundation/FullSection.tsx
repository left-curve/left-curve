import { twMerge } from "@left-curve/foundation";
import { View } from "react-native";
import { LinearGradient } from "expo-linear-gradient";
import { useTheme } from "~/hooks/useTheme";
import type { PropsWithChildren } from "react";
interface FullSectionProps {
  lightGradient?: string[];
  darkGradient?: string[];
  direction?: { start?: { x: number; y: number }; end?: { x: number; y: number } };
  className?: string;
}

export const FullSection: React.FC<PropsWithChildren<FullSectionProps>> = ({
  children,
  lightGradient = ["transparent", "transparent"],
  darkGradient = ["transparent", "transparent"],
  direction,
  className,
}) => {
  const { theme } = useTheme();
  const colorsArr = theme === "dark" ? darkGradient : lightGradient;
  const colors: [string, string, ...string[]] = [colorsArr[0], colorsArr[1]];

  return (
    <LinearGradient
      start={direction?.start ?? { x: 0.2, y: 0.0 }}
      end={direction?.end ?? { x: 0.9, y: 1.0 }}
      colors={colors}
    >
      <View className={twMerge("flex-1 w-full h-svh flex pt-4", className)}>{children}</View>
    </LinearGradient>
  );
};
