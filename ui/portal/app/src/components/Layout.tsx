import React, { PropsWithChildren } from "react";
import { View } from "react-native";
import { useTheme } from "~/hooks/useTheme";

export const Layout: React.FC<PropsWithChildren> = ({ children }) => {
  const { themeSchema } = useTheme();
  return (
    <View
      className={`flex-1 bg-surface-primary-rice text-primary-900 diatype-m-medium ${themeSchema}`}
    >
      {children}
    </View>
  );
};
