import React from "react";
import { Pressable, View } from "react-native";
import { IconSearch } from "./icons/IconSearch";
import { GlobalText } from "./GlobalText";
import { TextLoop } from "./TextLoop";
import { Button } from "./Button";
import { IconWallet } from "./icons/IconWallet";
import { useRouter } from "expo-router";

export const Menu: React.FC = () => {
  const { navigate } = useRouter();
  return (
    <View className="fixed bottom-0 lg:top-0 left-0 right-0 z-50 transition-all bg-transparent shadow-none lg:fixed min-h-10">
      <View
        accessibilityRole="header"
        className="w-full flex flex-row items-center justify-between gap-4 p-4"
      >
        <Pressable
          onPress={() => navigate("/search")}
          className="flex-1 h-[44px] flex justify-center p-2  pl-4 shadow shadow-btn-shadow-gradient bg-surface-secondary-rice rounded-md"
        >
          <View className="relative flex flex-row gap-2 items-center">
            <IconSearch className="text-tertiary-500" />
            <View className="flex flex-row gap-1 items-center relative">
              <GlobalText>Search for</GlobalText>
              <TextLoop
                texts={["blocks", "applets", "accounts", "transactions", "usernames", "tokens"]}
              />
            </View>
          </View>
        </Pressable>
        <Button
          variant="utility"
          size="icon"
          onPress={() => navigate("/signin")}
          leftIcon={<IconWallet />}
        />
      </View>
    </View>
  );
};
