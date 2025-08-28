import { useTheme } from "./hooks/useTheme";

import { twMerge } from "@left-curve/applets-kit";
import { SafeAreaView } from "react-native-safe-area-context";

import type { PropsWithChildren } from "react";
import type React from "react";

export const AppTheme: React.FC<PropsWithChildren> = ({ children }) => {
  const { theme } = useTheme();
  return (
    <SafeAreaView
      className={twMerge(
        theme,
        "flex-1 bg-surface-primary-rice text-primary-900 diatype-m-medium relative",
      )}
    >
      {children}
    </SafeAreaView>
  );
};
