import { twMerge } from "@left-curve/applets-kit";
import { View } from "react-native";
import { Button, GlobalText, IconAddCross, IconAlert, iconColors, IconSun } from "~/components";
import { useTheme } from "~/hooks/useTheme";

export default function HomeScreen() {
  const { theme, setThemeSchema } = useTheme();
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8">
      <GlobalText>Theme: {theme}</GlobalText>

      <Button
        variant="utility"
        size="md"
        onPress={() => setThemeSchema(theme === "light" ? "dark" : "light")}
      >
        <IconSun size={28} className="w-8 h-8" color={iconColors[theme]["utility"]} /> Set Theme
      </Button>
    </View>
  );
}
