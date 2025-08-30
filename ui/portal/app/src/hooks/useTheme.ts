import { useColorScheme } from "react-native";
import { useNativeStorage } from "./useNativeStorage";

export type ThemesSchema = "dark" | "light" | "system";
export type Themes = "dark" | "light";

export type UseThemeReturnType = {
  theme: Themes;
  themeSchema: ThemesSchema;
  setThemeSchema: (theme: ThemesSchema) => void;
};

export function useTheme(): UseThemeReturnType {
  const preferredSchema = useColorScheme();

  const [themeSchema, setThemeSchema] = useNativeStorage<ThemesSchema>("app.theme", {
    initialValue: "system",
    sync: true,
  });

  const theme = themeSchema === "system" ? preferredSchema || "dark" : themeSchema;

  return { theme, themeSchema, setThemeSchema };
}
