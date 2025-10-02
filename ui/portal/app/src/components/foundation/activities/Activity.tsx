import { useApp } from "@left-curve/applets-kit";
import { useActivities } from "@left-curve/store";
import { useCallback, useRef } from "react";

import { lazy, Suspense } from "react";
import { formatActivityTimestamp, twMerge } from "@left-curve/applets-kit";

import { Pressable, View, Text, type GestureResponderEvent } from "react-native";
import type { Activities, ActivityRecord } from "@left-curve/store";
import { ActivityTransfer } from "./Transfer";
import { ActivityNewAccount } from "./NewAccount";
import { ActivityOrderCreated } from "./OrderCreated";
import { ActivityOrderFilled } from "./OrderFilled";
import { ActivityOrderCanceled } from "./OrderCanceled";
import { IconClose } from "../icons/IconClose";

import type React from "react";
import type {
  ForwardRefExoticComponent,
  LazyExoticComponent,
  PropsWithoutRef,
  RefAttributes,
} from "react";

type CardFC = ForwardRefExoticComponent<
  PropsWithoutRef<{ activity: ActivityRecord<keyof Activities> }> & RefAttributes<ActivityRef>
>;

const activities: Record<keyof Activities, LazyExoticComponent<CardFC>> = {
  transfer: lazy(() => Promise.resolve({ default: ActivityTransfer as CardFC })),
  account: lazy(() => Promise.resolve({ default: ActivityNewAccount as CardFC })),
  orderCreated: lazy(() => Promise.resolve({ default: ActivityOrderCreated as CardFC })),
  orderFilled: lazy(() => Promise.resolve({ default: ActivityOrderFilled as CardFC })),
  orderCanceled: lazy(() => Promise.resolve({ default: ActivityOrderCanceled as CardFC })),
};

export type ActivityProps = {
  activity: ActivityRecord<keyof Activities>;
};

export type ActivityRef = {
  onPress: (event: GestureResponderEvent) => void;
};

export const Activity: React.FC<ActivityProps> = ({ activity }) => {
  const activityRef = useRef<ActivityRef | null>(null);
  const { settings } = useApp();
  const { dateFormat } = settings;
  const { deleteActivityRecord } = useActivities();
  const { id, createdAt, type } = activity;

  const ActivityCard = activities[type as keyof typeof activities];

  const handlePress = useCallback((e: GestureResponderEvent) => {
    activityRef.current?.onPress?.(e);
  }, []);

  const handleDeletePress = useCallback(
    (e: GestureResponderEvent) => {
      e.stopPropagation();
      deleteActivityRecord(id);
    },
    [deleteActivityRecord, id],
  );

  return (
    <Suspense>
      <Pressable
        onPress={handlePress}
        className={twMerge(
          "relative flex flex-row items-end justify-between gap-2 p-2 rounded-lg max-w-full bg-transparent",
        )}
        accessibilityRole="button"
      >
        <ActivityCard activity={activity} ref={activityRef} />

        <View className="min-w-fit items-center">
          <Pressable
            onPress={handleDeletePress}
            className="absolute top-1 right-1"
            accessibilityRole="button"
            accessibilityLabel="remove-activity"
          >
            <IconClose className="w-6 h-6" />
          </Pressable>

          <Text className="diatype-sm-medium text-ink-tertiary-500">
            {formatActivityTimestamp(new Date(createdAt), dateFormat.replace(/\/yyyy|yyyy\//g, ""))}
          </Text>
        </View>
      </Pressable>
    </Suspense>
  );
};
