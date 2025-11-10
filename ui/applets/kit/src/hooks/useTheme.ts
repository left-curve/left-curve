import { useStorage } from "@left-curve/store";
import { useEffect } from "react";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

export type UseThemeReturnType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

const getPreferredScheme = (): Themes => {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
};

export function useTheme(): UseThemeReturnType {
  const [themeSchema, setThemeSchema] = useStorage<ThemesSchema>("app.theme", {
    initialValue: "system",
    sync: true,
  });

  const theme = themeSchema === "system" ? getPreferredScheme() : themeSchema;

  useEffect(() => {
    const root = window?.document.documentElement;

    root.classList.remove("light", "dark");

    root.classList.add(theme);
  }, [theme]);

  return { theme, themeSchema, setThemeSchema };
}
