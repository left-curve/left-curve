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

export const exampleNotifications: Notification[] = [
  {
    id: "1",
    title: "Received",
    message: "+12.05 ETH from 0x6caf21cd9f6D4c6eF7C...",
    timestamp: new Date(),
    icon: "user",
  },
  {
    id: "2",
    title: "Sent",
    message: "-12.05 ETH to 0x6caf21cd9f6D4c6eF7C...",
    timestamp: new Date(new Date().getTime() - 5 * 60 * 1000), // Hace 5 minutos
    icon: "user",
  },
  {
    id: "3",
    title: "Swapped",
    message: "12.05 ETH for 100.45 USDC",
    timestamp: new Date("2024-12-01T14:30:00"),
    icon: "user",
  },
  {
    id: "4",
    title: "Liquidity removed",
    message: "-503.05 ETH, 30.87 USDT",
    timestamp: new Date("2024-12-01T10:20:00"),
    icon: "liquidity",
  },
  {
    id: "5",
    title: "Liquidity added",
    message: "+503.05 ETH, 30.87 USDT",
    timestamp: new Date("2024-11-30T08:45:00"),
    icon: "liquidity",
  },
  {
    id: "6",
    title: "System Assistant",
    message: "Dango is under maintenance",
    timestamp: new Date("2024-11-29T15:00:00"),
    icon: "system",
  },
  {
    id: "7",
    title: "Swapped",
    message: "8.00 ETH for 80.00 USDC",
    timestamp: new Date(new Date().getTime() - 2 * 60 * 60 * 1000), // Hace 2 horas
    icon: "user",
  },
  {
    id: "8",
    title: "Received",
    message: "+5.00 ETH from 0x12345abcd...",
    timestamp: new Date("2024-11-28T12:30:00"),
    icon: "user",
  },
  {
    id: "9",
    title: "Sent",
    message: "-1.00 ETH to 0x67890efgh...",
    timestamp: new Date("2024-11-28T09:15:00"),
    icon: "user",
  },
  {
    id: "10",
    title: "Received",
    message: "+0.50 ETH from 0x98765zyxw...",
    timestamp: new Date(new Date().getTime() - 30 * 60 * 1000), // Hace 30 minutos
    icon: "user",
  },
];

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
