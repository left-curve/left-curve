import { View } from "react-native";
import { Landing } from "~/components/Landing";

export default function HomeScreen() {
  return (
    <View className="flex-1 flex items-center justify-center bg-surface-primary-rice w-full flex-col gap-8">
      {/* <GlobalText>Theme: {theme}</GlobalText>

      <Button
        variant="utility"
        size="md"
        onPress={() => setThemeSchema(theme === "light" ? "dark" : "light")}
        leftIcon={<IconSun />}
      >
        Set Theme
      </Button> */}
      <Landing>
        <Landing.Header />
      </Landing>
    </View>
  );
}
