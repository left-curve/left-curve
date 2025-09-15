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
import { useNotifications, type Notifications } from "~/hooks/useNotifications";

import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { IconClose, useApp } from "@left-curve/applets-kit";

import type React from "react";

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

  return format(timestamp, mask);
};

const notifications: Record<
  keyof Notifications,
  LazyExoticComponent<
    ForwardRefExoticComponent<PropsWithoutRef<NotificationProps> & RefAttributes<NotificationRef>>
  >
> = {
  transfer: lazy(() =>
    import("./Transfer").then(({ NotificationTransfer }) => ({
      default: NotificationTransfer,
    })),
  ),
  account: lazy(() =>
    import("./NewAccount").then(({ NotificationNewAccount }) => ({
      default: NotificationNewAccount,
    })),
  ),
  orderCreated: lazy(() =>
    import("./OrderCreated").then(({ NotificationOrderCreated }) => ({
      default: NotificationOrderCreated,
    })),
  ),
  orderFilled: lazy(() =>
    import("./OrderFilled").then(({ NotificationOrderFilled }) => ({
      default: NotificationOrderFilled,
    })),
  ),
  orderCanceled: lazy(() =>
    import("./OrderCanceled").then(({ NotificationOrderCanceled }) => ({
      default: NotificationOrderCanceled,
    })),
  ),
};

export type NotificationProps = {
  notification: Notification[keyof Notification];
};

export type NotificationRef = {
  onClick: (event: React.MouseEvent<HTMLDivElement>) => void;
};

export const Notification: React.FC<NotificationProps> = ({ notification }) => {
  const notificationRef = useRef<NotificationRef | null>(null);
  const { settings } = useApp();
  const { dateFormat } = settings;
  const { deleteNotification } = useNotifications();
  const { id, createdAt, type } = notification;

  const NotificationCard = notifications[type as keyof typeof notifications];

  const handleClick = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    const element = event.target as HTMLElement;
    if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
      return;
    }
    notificationRef.current?.onClick(event);
  }, []);

  return (
    <Suspense>
      <div
        className="flex relative items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-secondary-rice max-w-full group cursor-pointer"
        onClick={handleClick}
      >
        <NotificationCard notification={notification} ref={notificationRef} />
        <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center">
          <IconClose
            className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-1 remove-notification"
            onClick={() => deleteNotification(id)}
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
