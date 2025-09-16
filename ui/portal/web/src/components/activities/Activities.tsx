import { useMemo, useState } from "react";
import { useActivities } from "@left-curve/store";

import {
  formatDate,
  ResizerContainer,
  Spinner,
  twMerge,
  useApp,
  useInfiniteScroll,
} from "@left-curve/applets-kit";

import { Activity } from "./Activity";
import { AnimatePresence, motion } from "framer-motion";
import { isToday } from "date-fns";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
import type { ActivityRecord } from "@left-curve/store";

type ActivitiesProps = {
  className?: string;
  activitiesPerCall?: number;
};

export const Activities: React.FC<ActivitiesProps> = (props) => {
  const { className, activitiesPerCall = 5 } = props;
  const { settings } = useApp();
  const { dateFormat } = settings;

  const [activitiesVisible, setActivitiesVisible] = useState(activitiesPerCall);
  const { userActivities, hasActivities, totalActivities } = useActivities();

  const activities: Record<string, ActivityRecord[]> = useMemo(() => {
    return [...userActivities]
      .reverse()
      .slice(0, activitiesVisible)
      .sort((a, b) => +b.createdAt - +a.createdAt)
      .reduce((acc, activity) => {
        const dateKey = isToday(activity.createdAt)
          ? "Today"
          : formatDate(activity.createdAt, dateFormat);

        if (!acc[dateKey]) {
          acc[dateKey] = [];
        }
        acc[dateKey].push(activity);
        return acc;
      }, Object.create({}));
  }, [userActivities, activitiesVisible]);

  const hasMoreActivities = activitiesVisible < totalActivities;

  const { loadMoreRef } = useInfiniteScroll(() => {
    setActivitiesVisible((prev) => Math.min(prev + activitiesPerCall, totalActivities));
  }, hasMoreActivities);

  if (!hasActivities) {
    return (
      <div className="px-4 flex flex-col gap-6 items-center">
        <img
          src="/images/emojis/detailed/hamster.svg"
          alt="hamster"
          className="mx-auto h-[125px] w-auto"
        />
        <div className="flex flex-col gap-2 items-center text-center">
          <p className="exposure-m-italic">{m["activities.noActivities.title"]()}</p>
          <p className="text-tertiary-500 diatype-m-bold">
            {m["activities.noActivities.description"]()}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <ResizerContainer
        layoutId="activities"
        className={twMerge("bg-transparent py-1 px-1 rounded-xl", className)}
      >
        <AnimatePresence mode="wait">
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            {Object.entries(activities).map(([dateKey, n]) => (
              <motion.div key={dateKey}>
                <p className="text-sm text-tertiary-500 mx-2 my-1">{dateKey}</p>
                <div className="flex flex-col gap-2 max-w-full">
                  {n.map((activity) => (
                    <Activity key={activity.id} activity={activity} />
                  ))}
                </div>
              </motion.div>
            ))}
            {hasMoreActivities ? (
              <div ref={loadMoreRef} className="flex justify-center py-2">
                <Spinner color="pink" />
              </div>
            ) : null}
          </motion.div>
        </AnimatePresence>
      </ResizerContainer>
    </div>
  );
};
