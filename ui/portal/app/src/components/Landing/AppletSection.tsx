import { useRouter } from "expo-router";

import { GlobalText, IconAddCross, ShadowContainer } from "../foundation";
import { View, Text, Pressable } from "react-native";

import { APPLETS, ASSETS } from "~/constants";

import type React from "react";
import type { AppletMetadata } from "@left-curve/store/types";

interface AppletSquareProps {
  applet: AppletMetadata;
}
const AppletSquare: React.FC<AppletSquareProps> = ({ applet }) => {
  const { push } = useRouter();
  const { id, title, path } = applet;

  const AppletAsset = ASSETS[applet.id as keyof typeof ASSETS];
  const AppletImage = AppletAsset ? AppletAsset.default : null;

  return (
    <View
      key={`applets.section.${id}`}
      className="w-1/3 md:w-1/4 items-center flex flex-col gap-2 mb-4"
    >
      <ShadowContainer>
        <Pressable
          onPress={() => push(path)}
          accessibilityRole="button"
          accessibilityLabel={title}
          className="h-20 w-20 rounded-xl p-2.5 bg-surface-primary-red active:opacity-80 flex items-center justify-center"
        >
          <AppletImage width="56" height="56" />
        </Pressable>
      </ShadowContainer>

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
  const { push } = useRouter();

  return (
    <View className="w-full flex flex-row flex-wrap">
      {Object.values(APPLETS).map((applet) => (
        <AppletSquare key={`applets.section.${applet.id}`} applet={applet} />
      ))}

      <View className="w-1/3 md:w-1/4  items-center mb-4">
        <ShadowContainer>
          <Pressable
            onPress={() => push("/search")}
            accessibilityRole="button"
            accessibilityLabel="Add applet"
            className="h-20 w-20 rounded-xl p-2.5 border border-outline-tertiary-rice bg-surface-primary-rice items-center justify-center"
          >
            <IconAddCross className="w-8 h-8 text-outline-tertiary-rice" />
          </Pressable>
        </ShadowContainer>
        <Text className="min-h-6" />
      </View>
    </View>
  );
};
