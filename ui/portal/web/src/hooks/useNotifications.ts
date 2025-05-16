import { useStorage } from "@left-curve/store";
import { format, isToday } from "date-fns";
import { useCallback, useMemo } from "react";
import type { Notifications } from "~/app.provider";

type UseNotificationsParameters = {
  maxNotifications?: number;
};

export function useNotifications(parameters: UseNotificationsParameters) {
  const { maxNotifications } = parameters;

  const [__notifications__, setNotifications] = useStorage<
    { type: string; data: unknown; createdAt: number }[]
  >("app.notifications", { initialValue: [], version: 0.1 });
  const pushNotification = useCallback(
    (notification: { type: string; data: unknown; createdAt: number }) => {
      setNotifications((prev) => [...prev, notification]);
    },
    [],
  );

  const hasNotifications = __notifications__.length > 0;

  const notifications: Record<string, Notifications[]> = useMemo(
    () =>
      [...__notifications__]
        .reverse()
        .slice(0, maxNotifications || __notifications__.length)
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
        }, Object.create({})),
    [maxNotifications, __notifications__],
  );

  return {
    notifications,
    pushNotification,
    hasNotifications,
  };
}
