import { forwardRef, useImperativeHandle } from "react";
import { View, Text, Image, Pressable } from "react-native";
import { useRouter } from "expo-router";

import { useConfig } from "@left-curve/store";
import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { twMerge, useApp } from "@left-curve/applets-kit";

import type { ActivityRecord } from "@left-curve/store";

import { IconSent } from "../icons/IconSent";
import { IconReceived } from "../icons/IconReceived";
import { PairAssets } from "../PairAssets";
import { AddressVisualizer } from "../AddressVisualizer";
import type { ActivityRef } from "./Activity";

type ActivityTransferProps = {
  activity: ActivityRecord<"transfer">;
  className?: string;
};

export const ActivityTransfer = forwardRef<ActivityRef, ActivityTransferProps>(
  ({ activity, className }, ref) => {
    const { settings } = useApp();
    const router = useRouter();
    const { getCoinInfo } = useConfig();

    const { coins, type, fromAddress, toAddress } = activity.data;
    const { formatNumberOptions } = settings;

    const isSent = type === "sent";
    const Icon = isSent ? IconSent : IconReceived;

    useImperativeHandle(ref, () => ({
      onPress: () => {
        /* { showModal(Modals.ActivityTransfer, { navigate, blockHeight, txHash, coins, action: type, from: fromAddress, to: toAddress, time: createdAt, }); } */
        console.log("show ActivityTransfer modal");
      },
    }));

    const onNavigate = (url: string) => {
      router.push(url.startsWith("/") ? url : `/${url}`);
    };

    return (
      <Pressable
        className={twMerge(
          "flex flex-row items-start gap-2 max-w-full overflow-hidden",
          "active:opacity-80",
          className,
        )}
        accessibilityRole="button"
        onPress={() => ref && (ref as any).current?.onPress?.()}
      >
        <View className="items-center justify-center bg-surface-quaternary-rice min-w-7 min-h-7 w-7 h-7 rounded-sm">
          <Icon
            className={twMerge(isSent ? "text-primitives-red-light-600" : "text-brand-green")}
          />
        </View>

        <View className="flex-1 flex-col max-w-[100%] overflow-hidden">
          <Text className="diatype-m-medium text-ink-secondary-700">Transfer</Text>

          <View className="flex flex-col items-start">
            {Object.entries(coins).map(([denom, amount]) => {
              const coin = getCoinInfo(denom);
              const formatted = formatNumber(
                formatUnits(amount, coin.decimals),
                formatNumberOptions,
              );

              return (
                <View
                  key={denom}
                  className={twMerge(
                    "diatype-m-bold flex flex-row items-center justify-center gap-1",
                    type === "received" ? "text-status-success" : "text-status-fail",
                  )}
                >
                  <View>
                    {coin.type === "lp" ? (
                      <PairAssets assets={[coin.base, coin.quote]} />
                    ) : (
                      <Image source={{ uri: coin.logoURI }} className="w-5 h-5 min-w-5 min-h-5" />
                    )}
                  </View>

                  <Text
                    className={twMerge(
                      "diatype-m-bold",
                      type === "received" ? "text-status-success" : "text-status-fail",
                    )}
                  >
                    {`${isSent ? "−" : "+"}${formatted}  ${coin.symbol}`}
                  </Text>
                </View>
              );
            })}
          </View>

          <View className="flex flex-col diatype-m-medium text-ink-tertiary-500 items-start gap-1 mt-1">
            <View className="flex flex-row flex-wrap items-center gap-1">
              <Text>From</Text>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={fromAddress}
                withIcon
                onClick={onNavigate}
              />
            </View>

            <View className="flex flex-row flex-wrap items-center gap-1">
              <Text>To</Text>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={toAddress}
                withIcon
                onClick={onNavigate}
              />
            </View>
          </View>
        </View>
      </Pressable>
    );
  },
);
