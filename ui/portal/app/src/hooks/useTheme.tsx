// hooks/useTheme.ts
import type React from "react";
import { createContext, type PropsWithChildren, useContext, useMemo, useState } from "react";
import { storage } from "../../storage.config";
import { useColorScheme } from "react-native";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

type ThemeContextType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

const THEME_KEY = "app.theme";

export const ThemeProvider: React.FC<PropsWithChildren> = ({ children }) => {
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

  const value = useMemo<ThemeContextType>(
    () => ({ theme, themeSchema, setThemeSchema }),
    [theme, themeSchema],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
};

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used inside ThemeProvider");
  return ctx;
}
