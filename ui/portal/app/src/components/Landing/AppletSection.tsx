import { useRouter } from "expo-router";

import { GlobalText, IconAddCross } from "../foundation";
import { View, Text, Pressable } from "react-native";

import { APPLETS, ASSETS } from "~/constants";

import type React from "react";
import type { AppletMetadata } from "@left-curve/store/src/types";

interface AppletSquareProps {
  applet: AppletMetadata;
}

const AppletSquare: React.FC<AppletSquareProps> = ({ applet }) => {
  const router = useRouter();
  const AppletImage = ASSETS[applet.id as keyof typeof ASSETS].default;
  const { id, title, path } = applet;
  return (
    <View key={`applets.section.${id}`} className="items-center flex-col flex gap-2 w-[96px]">
      <Pressable
        onPress={() => router.push(path)}
        accessibilityRole="button"
        accessibilityLabel={title}
        className="h-20 w-20 rounded-xl p-2.5 shadow-account-card bg-primary-red active:opacity-80 flex items-center justify-center"
      >
        <AppletImage width="44" height="44" />
      </Pressable>

      <GlobalText
        ellipsizeMode="clip"
        className="diatype-sm-bold px-1 text-center"
        numberOfLines={2}
        adjustsFontSizeToFit
      >
        {title}
      </GlobalText>
    </View>
  );
};

export const AppletsSection: React.FC = () => {
  const router = useRouter();

  return (
    <View className="w-full flex flex-row items-start flex-wrap gap-4">
      {Object.values(APPLETS).map((applet) => (
        <AppletSquare key={`applets.section.${applet.id}`} applet={applet} />
      ))}

      <View className="w-[96px] flex items-center">
        <Pressable
          onPress={() => router.push("/search")}
          accessibilityRole="button"
          accessibilityLabel="Add applet"
          className="h-20 w-20 rounded-xl p-2.5 shadow shadow-account-card border border-tertiary-rice  bg-foreground-primary-rice items-center justify-center"
        >
          <IconAddCross className="w-8 h-8 text-tertiary-rice" />
        </Pressable>
        <Text className="min-h-6" />
      </View>
    </View>
  );
};
