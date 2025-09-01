import { View } from "react-native";
import { Landing } from "~/components/Landing";
import { APPLETS, ASSETS } from "~/constants";

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
        <View>
          {Object.values(APPLETS).map((applet) => {
            const { default: AppletImage } = ASSETS[applet.id as keyof typeof ASSETS];
            return (
              <View key={applet.id}>
                <AppletImage width="40" height="40" />
              </View>
            );
          })}
        </View>
      </Landing>
    </View>
  );
}
