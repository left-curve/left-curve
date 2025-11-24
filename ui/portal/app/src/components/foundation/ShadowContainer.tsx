import type React from "react";
import { Shadow } from "react-native-shadow-2";
import type { PropsWithChildren } from "react";
import { useTheme } from "~/hooks/useTheme";
import type { StyleProp, ViewStyle } from "react-native";

interface ShadowContainerProps {
  borderRadius?: number;
  style?: ViewStyle;
}

export const ShadowContainer: React.FC<PropsWithChildren<ShadowContainerProps>> = ({
  children,
  borderRadius = 20,
  style,
}) => {
  const { themeSchema } = useTheme();
  const shadowStyle = { borderRadius, width: "100%", ...style } as StyleProp<ViewStyle>;
  return (
    <Shadow
      startColor={themeSchema === "light" ? "rgba(241,219,186,0.5)" : "rgba(25,25,24,0.4)"}
      distance={2}
      offset={[0, 2]}
      style={shadowStyle}
    >
      <Shadow
        startColor={themeSchema === "light" ? "rgba(171,158,138,0.2)" : "rgba(25,25,24,0.5)"}
        distance={6}
        offset={[0, 2]}
        style={shadowStyle}
      >
        {children}
      </Shadow>
    </Shadow>
  );
};
