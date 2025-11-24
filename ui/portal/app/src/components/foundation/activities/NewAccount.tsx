import { useRouter } from "expo-router";

import { forwardRef, useImperativeHandle } from "react";

import { View, type GestureResponderEvent } from "react-native";
import { Badge } from "../Badge";
import { AddressVisualizer } from "../AddressVisualizer";
import { IconNewAccount } from "../icons/IconNewAccount";

import type { ActivityRecord } from "@left-curve/store";
import { GlobalText } from "../GlobalText";

export type ActivityRef = {
  onPress: (event: GestureResponderEvent) => void;
};

type ActivityAccountProps = {
  activity: ActivityRecord<"account">;
};

export const ActivityNewAccount = forwardRef<ActivityRef, ActivityAccountProps>(
  ({ activity }, ref) => {
    const { navigate } = useRouter();
    const { address, accountType } = activity.data;

    const navigateToAccount = () => navigate("account");
    useImperativeHandle(ref, () => ({
      onPress: () => navigate("account"),
    }));

    return (
      <View className="flex flex-row items-start gap-2 max-w-full overflow-hidden">
        <View className="flex justify-center items-center bg-surface-primary-gray w-7 h-7 rounded-sm">
          <IconNewAccount className="text-brand-green h-4 w-4" />
        </View>

        <View className="flex flex-col max-w-[100%] overflow-hidden">
          <View className="flex flex-row justify-center items-center gap-2 diatype-m-medium text-ink-secondary-700 capitalize">
            <GlobalText>Account created</GlobalText>
            <Badge classNames={{ text: "capitalize" }} text={accountType} />
          </View>

          <AddressVisualizer
            classNames={{
              text: "diatype-m-medium text-ink-tertiary-500",
            }}
            address={address}
            withIcon
            onClick={navigateToAccount}
          />
        </View>
      </View>
    );
  },
);
