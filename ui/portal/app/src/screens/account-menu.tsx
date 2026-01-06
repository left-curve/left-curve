import { useRouter } from "expo-router";
import { useState } from "react";
import { Image, View } from "react-native";
import {
  AccountCard,
  Activities,
  Button,
  GlobalText,
  IconAddCross,
  IconLogOut,
  IconSwitch,
  Tabs,
} from "~/components/foundation";

export default function AccountMenuScreen() {
  const { navigate } = useRouter();
  const [activeTab, setActiveTab] = useState("wallet");

  return (
    <View className="flex-1 flex bg-surface-primary-rice w-full flex-col gap-8 px-4 py-6">
      <View className="flex flex-col w-full gap-5">
        <AccountCard
          account={{
            index: 10,
            params: {
              single: {
                owner: 1,
              },
            },
            address: "0x75b89bd4a0e12b45dd12a6e12550aed2b8fd858f",
            username: "cookie",
            type: "single",
          }}
          balance={"120"}
        />
        <View className="flex-row gap-2 w-full">
          <View className="flex-1">
            <Button
              size="lg"
              leftIcon={<IconAddCross className="w-5 h-5" />}
              classNames={{ base: "w-full" }}
            >
              <GlobalText>Fund</GlobalText>
            </Button>
          </View>
          <View className="flex-1">
            <Button
              variant="secondary"
              size="lg"
              leftIcon={<IconSwitch className="w-5 h-5" />}
              classNames={{ base: "w-full" }}
            >
              <GlobalText>Switch</GlobalText>
            </Button>
          </View>
          <Button
            variant="secondary"
            size="icon"
            onPress={() => navigate("/signin")}
            leftIcon={<IconLogOut className="w-5 h-5" />}
          />
        </View>
        <View className="flex-row gap-2 w-full min-h-[30px]">
          <Tabs
            color="line-red"
            selectedTab={activeTab}
            keys={["wallet", "activities"]}
            fullWidth
            onTabChange={setActiveTab}
          />
        </View>
        <View>
          {activeTab === "wallet" ? (
            <View className="px-4 flex flex-col gap-6 items-center">
              <Image
                source={require("@left-curve/foundation/images/emojis/detailed/hamster.svg")}
                resizeMode="contain"
                style={{ height: 125, width: 125 }}
              />
              <View className="flex flex-col gap-2 items-center text-center">
                <GlobalText className="exposure-m-italic">No tokens yet</GlobalText>
              </View>
            </View>
          ) : (
            <Activities />
          )}
        </View>
      </View>
    </View>
  );
}
