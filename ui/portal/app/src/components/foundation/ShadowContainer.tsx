import type React from "react";
import { Shadow } from "react-native-shadow-2";
import type { PropsWithChildren } from "react";
import { useTheme } from "~/hooks/useTheme";

interface ShadowContainerProps {
  borderRadius?: number;
}

export const ShadowContainer: React.FC<PropsWithChildren<ShadowContainerProps>> = ({
  children,
  borderRadius = 20,
}) => {
  const { themeSchema, setThemeSchema } = useTheme();
  return (
    <Shadow
      startColor={themeSchema === "light" ? "rgba(241,219,186,0.5)" : "rgba(25,25,24,0.4)"}
      endColor={themeSchema === "light" ? "rgba(171,158,138,0.4)" : "rgba(25,25,24,0.5)"}
      distance={2}
      offset={[0, 2]}
      style={{ borderRadius, width: "100%" }}
    >
      {children}
    </Shadow>
  );
};
