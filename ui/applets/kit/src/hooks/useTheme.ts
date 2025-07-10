import { useStorage } from "@left-curve/store";
import { useCallback, useEffect } from "react";

export type ThemeType = "dark" | "light";

export type UseThemeReturnType = {
  theme: ThemeType;
  setTheme: (theme: ThemeType) => void;
  toggleTheme: () => void;
};

const getInitialTheme = (): ThemeType => {
  /* if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } */
  return "light";
};

export function useTheme(): UseThemeReturnType {
  const [theme, setTheme] = useStorage<ThemeType>("app.theme", {
    initialValue: getInitialTheme(),
  });

  useEffect(() => {
    const root = window.document.documentElement;

    root.classList.remove("light", "dark");

    /* root.classList.add(theme); */
    root.classList.add("light");
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((prevTheme) => (prevTheme === "dark" ? "light" : "dark"));
  }, [setTheme]);

  const setThemeInfo = useCallback(
    (newTheme: ThemeType) => {
      setTheme(newTheme);
    },
    [setTheme],
  );

  return {
    theme,
    setTheme: setThemeInfo,
    toggleTheme,
  };
}
