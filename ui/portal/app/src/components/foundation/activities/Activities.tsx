import type React from "react";
import { useMemo, useState, useCallback } from "react";
import { View, SectionList, ActivityIndicator, type ListRenderItemInfo } from "react-native";
import { useActivities } from "@left-curve/store";
import { isToday } from "date-fns";
import { twMerge, formatDate, useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import HamsterEmoji from "@left-curve/foundation/images/emojis/detailed/hamster.svg";

import { Activity } from "./Activity";

import type { ActivityRecord } from "@left-curve/store";
import { GlobalText } from "../GlobalText";

type ActivitiesProps = {
  className?: string;
  activitiesPerCall?: number;
};

type Section = { title: string; data: ActivityRecord[] };

export const Activities: React.FC<ActivitiesProps> = ({ className, activitiesPerCall = 5 }) => {
  const { settings } = useApp();
  const { dateFormat } = settings;

  const { userActivities, hasActivities, totalActivities } = useActivities();
  const [visible, setVisible] = useState(activitiesPerCall);
  const hasMore = visible < totalActivities;

  const sections: Section[] = useMemo(() => {
    const limited = [...userActivities]
      .reverse()
      .slice(0, visible)
      .sort((a, b) => +b.createdAt - +a.createdAt);

    const map = new Map<string, ActivityRecord[]>();
    for (const activity of limited) {
      const key = isToday(activity.createdAt)
        ? "Today"
        : formatDate(activity.createdAt, dateFormat);
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(activity);
    }

    return Array.from(map.entries()).map(([title, data]) => ({ title, data }));
  }, [userActivities, visible, dateFormat]);

  const loadMore = useCallback(() => {
    if (!hasMore) return;
    setVisible((prev) => Math.min(prev + activitiesPerCall, totalActivities));
  }, [hasMore, activitiesPerCall, totalActivities]);

  if (!hasActivities) {
    return (
      <View className="px-4 py-20 flex flex-col gap-6 items-center">
        <HamsterEmoji className="h-32 w-32" />
        <View className="flex flex-col gap-2 items-center text-center">
          <GlobalText className="exposure-m-italic">
            {m["activities.noActivities.title"]()}
          </GlobalText>
          <GlobalText className="text-ink-tertiary-500 diatype-m-bold">
            {m["activities.noActivities.description"]()}
          </GlobalText>
        </View>
      </View>
    );
  }

  const renderItem = ({ item }: ListRenderItemInfo<ActivityRecord>) => (
    <View className="mb-2">
      <Activity activity={item} />
    </View>
  );

  const renderSectionHeader = ({ section }: { section: Section }) => (
    <GlobalText className="text-sm text-ink-tertiary-500 mx-2 my-1">{section.title}</GlobalText>
  );

  const Separator: React.FC = () => <View className="h-6" />;

  return (
    <View className={twMerge("flex flex-col h-[52vh]", className)}>
      <SectionList
        sections={sections}
        keyExtractor={(item) => item.id}
        renderItem={renderItem}
        renderSectionHeader={renderSectionHeader}
        contentContainerStyle={{ paddingHorizontal: 4, paddingVertical: 4 }}
        onEndReachedThreshold={0.4}
        onEndReached={loadMore}
        ListFooterComponent={
          hasMore ? (
            <View className="flex justify-center py-2">
              <ActivityIndicator />
            </View>
          ) : null
        }
        SectionSeparatorComponent={Separator}
        initialNumToRender={10}
        windowSize={10}
        removeClippedSubviews
        stickySectionHeadersEnabled={false}
      />
    </View>
  );
};
