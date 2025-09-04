import { useRouter } from "expo-router";

import { View, Pressable } from "react-native";
import { GlobalText, IconChevronDown } from "~/components/foundation"; // tu icono adaptado a RN

import { twMerge } from "@left-curve/foundation";

import type React from "react";

type MobileTitleProps = {
  title: string;
  className?: string;
};

export const MobileTitle: React.FC<MobileTitleProps> = ({ title, className }) => {
  const { back } = useRouter();

  return (
    <View className={twMerge("flex flex-row gap-2 items-center self-start", className)}>
      <Pressable onPress={() => back()} className="p-2 rotate-90" accessibilityRole="button">
        <IconChevronDown className="w-5 h-5 text-primary-900" />
      </Pressable>

      <GlobalText className="h3-bold text-primary-900">{title}</GlobalText>
    </View>
  );
};
