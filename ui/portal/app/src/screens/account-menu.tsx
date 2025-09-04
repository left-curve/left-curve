import { useRouter } from "expo-router";
import { View } from "react-native";
import {
  AccountCard,
  Button,
  GlobalText,
  IconAddCross,
  IconLogOut,
  IconSwitch,
} from "~/components/foundation";

export default function AccountMenuScreen() {
  const { navigate } = useRouter();

  return (
    <View className="flex-1 flex bg-surface-primary-rice w-full flex-col gap-8 px-4 py-6">
      <View className="flex flex-col w-full gap-5">
        <AccountCard
          account={{
            index: 10,
            params: {
              spot: {
                owner: "cookie",
              },
            },
            address: "0x75b89bd4a0e12b45dd12a6e12550aed2b8fd858f",
            username: "cookie",
            type: "spot",
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
      </View>
    </View>
  );
}
