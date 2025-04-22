import { useApp } from "~/hooks/useApp";

import { twMerge } from "@left-curve/applets-kit";
import { format, isToday } from "date-fns";

import { m } from "~/paraglide/messages";

import type React from "react";
import type { Notifications } from "~/app.provider";
import { Notification } from "./Notification";

type NotificationListProps = {
  className?: string;
};

const notificationCard = {
  transfer: Notification.Transfer,
};

export const NotificationsList: React.FC<NotificationListProps> = ({ className }) => {
  const { notifications } = useApp();

  const sortedNotifications: Record<string, Notifications[]> = [...notifications]
    .sort((a, b) => b.createdAt - a.createdAt)
    .reduce((acc, notification) => {
      const dateKey = isToday(notification.createdAt)
        ? "Today"
        : format(notification.createdAt, "MM/dd/yyyy");

      if (!acc[dateKey]) {
        acc[dateKey] = [];
      }
      acc[dateKey].push(notification);
      return acc;
    }, Object.create({}));

  if (!notifications.length) {
    return (
      <div className="min-h-[19rem] flex flex-col gap-4 items-center justify-center px-4 py-6 text-center relative bg-[url('./images/notifications/bubble-bg.svg')] bg-[-11rem_4rem] bg-no-repeat">
        <img
          src="/images/notifications/no-notifications.svg"
          alt="no-notifications"
          className="h-[154px]"
        />
        <p className="exposure-m-italic">{m["notifications.noNotifications.title"]()}</p>
        <p className="diatype-m-bold text-gray-500">
          {m["notifications.noNotifications.description"]()}
        </p>
      </div>
    );
  }

  return (
    <div className={twMerge("bg-transparent py-2 px-1 rounded-xl shadow-lg", className)}>
      {Object.entries(sortedNotifications).map(([dateKey, n]) => (
        <div key={dateKey}>
          <p className="text-sm text-gray-500 mx-2">{dateKey}</p>
          <div className="flex flex-col gap-2">
            {n.map((notification) => {
              const NotificationCard =
                notificationCard[notification.type as keyof typeof notificationCard];
              return (
                <NotificationCard key={notification.createdAt} notification={notification as any} />
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
};
