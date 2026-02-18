import { useRouter } from "expo-router";

import { Button } from "./Button";
import { TextLoop } from "./TextLoop";
import { GlobalText } from "./GlobalText";
import { Pressable, View } from "react-native";
import { IconSearch } from "./icons/IconSearch";
import { IconWallet } from "./icons/IconWallet";

import type React from "react";
import { ShadowContainer } from "./ShadowContainer";
import { useAccount } from "@left-curve/store";

export const Menu: React.FC = () => {
  const { navigate } = useRouter();
  const { account } = useAccount();

  return (
    <View className="absolute bottom-0 lg:top-0 left-0 right-0 z-50 transition-all bg-transparent shadow-none min-h-10">
      <View
        accessibilityRole="header"
        className="w-full flex flex-row items-center justify-between gap-4 p-4"
      >
        <View className="flex-1 h-[44px] rounded-md">
          <ShadowContainer style={{ borderRadius: 12, width: "100%" }}>
            <Pressable
              onPress={() => navigate("/search")}
              className="h-[44px] flex justify-center p-2 pl-4 bg-surface-secondary-rice rounded-md"
            >
              <View className="relative flex flex-row gap-2 items-center">
                <IconSearch className="text-ink-tertiary-500" />
                <View className="flex flex-row gap-1 items-center relative">
                  <GlobalText>Search for</GlobalText>
                  <TextLoop
                    texts={["blocks", "applets", "accounts", "transactions", "usernames", "tokens"]}
                  />
                </View>
              </View>
            </Pressable>
          </ShadowContainer>
        </View>
        <Button
          variant="utility"
          size="icon"
          onPress={() => navigate(account ? "/account-menu" : "/auth")}
          leftIcon={<IconWallet />}
        />
      </View>
    </View>
  );
};
