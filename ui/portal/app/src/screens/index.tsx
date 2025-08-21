import { View } from "react-native";
import { Button, GlobalText, iconColors, IconSun } from "~/components";
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
        leftIcon={<IconSun size={28} color={iconColors[theme]["utility"]} />}
      >
        Set Theme
      </Button>
    </View>
  );
}
