// hooks/useTheme.ts
import type React from "react";
import { createContext, PropsWithChildren, useContext, useEffect, useMemo, useState } from "react";
import { Appearance } from "react-native";
import { storage } from "../../storage.config";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

type ThemeContextType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);
const THEME_KEY = "app.theme";

function getPreferredScheme(): Themes {
  const cs = Appearance.getColorScheme();
  return cs === "dark" ? "dark" : "light";
}

export const ThemeProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const initialSchema = useMemo<ThemesSchema>(() => {
    const saved = storage.getString(THEME_KEY);
    return saved === "light" || saved === "dark" || saved === "system" ? saved : "system";
  }, []);

  const [themeSchema, setThemeSchemaState] = useState<ThemesSchema>(initialSchema);
  const [theme, setTheme] = useState<Themes>(
    initialSchema === "system" ? getPreferredScheme() : initialSchema,
  );

  useEffect(() => {
    storage.set(THEME_KEY, themeSchema);
    const next = themeSchema === "system" ? getPreferredScheme() : themeSchema;
    setTheme(next);
  }, [themeSchema]);

  useEffect(() => {
    const sub = Appearance.addChangeListener(({ colorScheme }) => {
      if (themeSchema === "system") {
        const next = colorScheme === "dark" ? "dark" : "light";
        setTheme(next);
      }
    });
    return () => sub.remove();
  }, [themeSchema]);

  return (
    <ThemeContext.Provider
      value={{
        theme,
        themeSchema,
        setThemeSchema: setThemeSchemaState,
      }}
    >
      {children}
    </ThemeContext.Provider>
  );
};

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme must be used inside ThemeProvider");
  return ctx;
}
