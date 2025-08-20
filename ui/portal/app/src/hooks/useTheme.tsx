import React, { createContext, useContext, useEffect, useState } from "react";
import { Appearance } from "react-native";
import AsyncStorage from "@react-native-async-storage/async-storage";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

type ThemeContextType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

function getPreferredScheme(): Themes {
  const colorScheme = Appearance.getColorScheme();
  return colorScheme === "dark" ? "dark" : "light";
}

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [themeSchema, setThemeSchemaState] = useState<ThemesSchema>("system");
  const [theme, setTheme] = useState<Themes>(getPreferredScheme());

  useEffect(() => {
    AsyncStorage.getItem("app.theme").then((saved) => {
      if (saved) {
        setThemeSchemaState(saved as ThemesSchema);
      }
    });
  }, []);

  useEffect(() => {
    AsyncStorage.setItem("app.theme", themeSchema);

    const nextTheme = themeSchema === "system" ? getPreferredScheme() : themeSchema;
    setTheme(nextTheme);
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
