import { lazy, Suspense } from "react";
import { useNotifications } from "~/hooks/useNotifications";

import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { IconClose } from "@left-curve/applets-kit";

import type React from "react";

const formatNotificationTimestamp = (timestamp: Date): string => {
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

  return format(timestamp, "MM/dd");
};

const notifications = {
  transfer: {
    component: lazy(() =>
      import("./Transfer").then(({ NotificationTransfer }) => ({
        default: NotificationTransfer,
      })),
    ),
  },
  account: {
    component: lazy(() =>
      import("./NewAccount").then(({ NotificationNewAccount }) => ({
        default: NotificationNewAccount,
      })),
    ),
  },
  orderCreated: {
    component: lazy(() =>
      import("./OrderCreated").then(({ NotificationOrderCreated }) => ({
        default: NotificationOrderCreated,
      })),
    ),
  },
  orderFilled: {
    component: lazy(() =>
      import("./OrderFilled").then(({ NotificationOrderFilled }) => ({
        default: NotificationOrderFilled,
      })),
    ),
  },
};

export type NotificationProps = {
  notification: Notification[keyof Notification];
};

export const Notification: React.FC<NotificationProps> = ({ notification }) => {
  const { deleteNotification } = useNotifications();
  const { id, createdAt, type } = notification;

  const NotificationCard = notifications[type as keyof typeof notifications]
    ?.component as React.FC<NotificationProps>;

  return (
    <Suspense>
      <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
        <NotificationCard notification={notification} />
        <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
          <IconClose
            className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px] remove-notification"
            onClick={() => deleteNotification(id)}
          />
          <p>{formatNotificationTimestamp(new Date(createdAt))}</p>
        </div>
      </div>
    </Suspense>
  );
};
