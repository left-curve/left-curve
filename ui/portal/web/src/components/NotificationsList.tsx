import { twMerge } from "@left-curve/applets-kit";
import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  formatDistanceToNow,
  isToday,
} from "date-fns";
import type React from "react";

interface Notification {
  id: string;
  title: string;
  message: string;
  timestamp: Date;
  icon?: string;
}

interface Props {
  notifications: Notification[];
  className?: string;
}

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

export const NotificationsList: React.FC<Props> = ({ notifications, className }) => {
  const sortedNotifications = [...notifications].sort(
    (a, b) => b.timestamp.getTime() - a.timestamp.getTime(),
  );

  const groupedNotifications: { [date: string]: Notification[] } = {};
  sortedNotifications.forEach((notification) => {
    const dateKey = isToday(notification.timestamp)
      ? "Today"
      : format(notification.timestamp, "MM/dd/yyyy");

    if (!groupedNotifications[dateKey]) {
      groupedNotifications[dateKey] = [];
    }
    groupedNotifications[dateKey].push(notification);
  });

  if (!notifications.length) {
    return (
      <div className="min-h-[19rem] flex flex-col gap-4 items-center justify-center px-4 py-6 text-center relative bg-[url('./images/notifications/bubble-bg.svg')] bg-[-11rem_4rem] bg-no-repeat">
        <img
          src="/images/notifications/no-notifications.svg"
          alt="no-notifications"
          className="w-[133px] h-[144px]"
        />
        <p className="exposure-m-italic">No notifications yet</p>
        <p className="diatype-m-bold text-gray-500">
          When you approve, trade, or transfer tokens, your transaction will appear here
        </p>
      </div>
    );
  }

  return (
    <div className={twMerge("bg-transparent py-2 px-1 rounded-xl shadow-lg", className)}>
      {Object.keys(groupedNotifications).map((dateKey) => (
        <div key={dateKey}>
          <p className="text-sm text-gray-500 mx-2">{dateKey}</p>
          <div className="flex flex-col gap-2">
            {groupedNotifications[dateKey].map((notification) => (
              <div
                key={notification.id}
                className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-rice-100"
              >
                <div className="flex items-start gap-2">
                  {notification.icon && (
                    <img
                      src={`/images/notifications/${notification.icon}.svg`}
                      alt="Icon"
                      className="w-6 h-6 rounded-full"
                    />
                  )}
                  <div className="flex flex-col">
                    <div className="diatype-m-medium text-gray-700">{notification.title}</div>
                    <div className="diatype-m-medium text-gray-500">{notification.message}</div>
                  </div>
                </div>
                <div className="diatype-sm-medium text-gray-500">
                  {formatNotificationTimestamp(notification.timestamp)}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};
