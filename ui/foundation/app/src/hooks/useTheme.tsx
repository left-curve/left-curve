import { useMemo, useState } from "react";
import { useColorScheme } from "react-native";

import { createContext } from "@left-curve/foundation-shared";

import type { PropsWithChildren } from "react";
import type React from "react";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

type ThemeContextType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

const [ThemeContextProvider, useTheme] = createContext<ThemeContextType>({
  name: "ThemeContext",
});

const THEME_KEY = "app.theme";

export const ThemeProvider: React.FC<PropsWithChildren<{ storage: any }>> = ({
  children,
  storage,
}) => {
  const systemSchema = (useColorScheme() ?? "light") as Themes;

  const [themeSchema, setThemeSchemaState] = useState<ThemesSchema>(() => {
    return (storage.getString(THEME_KEY) as ThemesSchema) ?? "system";
  });

  const theme = useMemo<Themes>(
    () => (themeSchema === "system" ? systemSchema : themeSchema),
    [themeSchema, systemSchema],
  );

  const setThemeSchema = (next: ThemesSchema) => {
    setThemeSchemaState(next);
    storage.set(THEME_KEY, next);
  };

  return (
    <ThemeContextProvider value={{ theme, themeSchema, setThemeSchema }}>
      {children}
    </ThemeContextProvider>
  );
};

export { useTheme };
