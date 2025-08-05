import { useStorage } from "@left-curve/store";
import { useEffect } from "react";

export type Themes = "dark" | "light" | "system";

export type UseThemeReturnType = {
  theme: Themes;
  setTheme: (theme: Themes) => void;
};

const getPreferredScheme = (): Themes => {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
};

export function useTheme(): UseThemeReturnType {
  const [theme, setTheme] = useStorage<Themes>("app.theme", {
    initialValue: "system",
    sync: true,
  });

  useEffect(() => {
    const root = window.document.documentElement;

    root.classList.remove("light", "dark");

    const isSystemTheme = theme === "system";

    root.classList.add(isSystemTheme ? getPreferredScheme() : theme);
  }, [theme]);

  return { theme, setTheme };
}
