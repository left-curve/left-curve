import {
  type ForwardRefExoticComponent,
  lazy,
  type LazyExoticComponent,
  type PropsWithoutRef,
  type RefAttributes,
  Suspense,
  useCallback,
  useRef,
} from "react";
import { useActivities } from "@left-curve/store";

import { differenceInDays, differenceInHours, differenceInMinutes, isToday } from "date-fns";

import { formatDate, IconClose, useApp } from "@left-curve/applets-kit";

import type React from "react";
import type { Activities, ActivityRecord } from "@left-curve/store";

const formatNotificationTimestamp = (timestamp: Date, mask: string): string => {
  const now = new Date();
  if (isToday(timestamp)) {
    const minutesDifference = differenceInMinutes(now, timestamp);
    if (minutesDifference < 1) {
      return "1m";
    }

    if (minutesDifference < 60) {
      return `${minutesDifference}m`;
    }

    const hoursDifference = differenceInHours(now, timestamp);
    if (hoursDifference < 24) {
      return `${hoursDifference}h`;
    }
  }

  const daysDifference = differenceInDays(now, timestamp);
  if (daysDifference === 1) {
    return "1d";
  }

  return formatDate(timestamp, mask);
};

const activities: Record<
  keyof Activities,
  LazyExoticComponent<
    ForwardRefExoticComponent<
      PropsWithoutRef<{ activity: ActivityRecord<keyof Activities> }> & RefAttributes<ActivityRef>
    >
  >
> = {
  transfer: lazy(() =>
    import("./Transfer").then(({ ActivityTransfer }) => ({
      default: ActivityTransfer as ForwardRefExoticComponent<{
        activity: ActivityRecord<keyof Activities>;
      }>,
    })),
  ),
  account: lazy(() =>
    import("./NewAccount").then(({ ActivityNewAccount }) => ({
      default: ActivityNewAccount as ForwardRefExoticComponent<{
        activity: ActivityRecord<keyof Activities>;
      }>,
    })),
  ),
  orderCreated: lazy(() =>
    import("./OrderCreated").then(({ ActivityOrderCreated }) => ({
      default: ActivityOrderCreated as ForwardRefExoticComponent<{
        activity: ActivityRecord<keyof Activities>;
      }>,
    })),
  ),
  orderFilled: lazy(() =>
    import("./OrderFilled").then(({ ActivityOrderFilled }) => ({
      default: ActivityOrderFilled as ForwardRefExoticComponent<{
        activity: ActivityRecord<keyof Activities>;
      }>,
    })),
  ),
  orderCanceled: lazy(() =>
    import("./OrderCanceled").then(({ ActivityOrderCanceled }) => ({
      default: ActivityOrderCanceled as ForwardRefExoticComponent<{
        activity: ActivityRecord<keyof Activities>;
      }>,
    })),
  ),
};

export type ActivityProps = {
  activity: ActivityRecord<keyof Activities>;
};

export type ActivityRef = {
  onClick: (event: React.MouseEvent<HTMLDivElement>) => void;
};

export const Activity: React.FC<ActivityProps> = ({ activity }) => {
  const activityRef = useRef<ActivityRef | null>(null);
  const { settings } = useApp();
  const { dateFormat } = settings;
  const { deleteActivityRecord } = useActivities();
  const { id, createdAt, type } = activity;

  const ActivityCard = activities[type as keyof typeof activities];

  const handleClick = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    const element = event.target as HTMLElement;
    if (element.closest(".address-visualizer") || element.closest(".remove-activity")) {
      return;
    }
    activityRef.current?.onClick(event);
  }, []);

  return (
    <Suspense>
      <div
        className="flex relative items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-secondary-rice max-w-full group cursor-pointer"
        onClick={handleClick}
      >
        <ActivityCard activity={activity} ref={activityRef} />
        <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center">
          <IconClose
            className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-1 remove-activity"
            onClick={() => deleteActivityRecord(id)}
          />
          <p>
            {formatNotificationTimestamp(
              new Date(createdAt),
              dateFormat.replace(/\/yyyy|yyyy\//g, ""),
            )}
          </p>
        </div>
      </div>
    </Suspense>
  );
};
