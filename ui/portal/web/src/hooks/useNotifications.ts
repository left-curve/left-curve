import { useAccount, useConfig, useStorage } from "@left-curve/store";
import { useCallback, useMemo } from "react";

import { uid } from "@left-curve/dango/utils";
import { format, isToday } from "date-fns";

import type { Address, UID, Username } from "@left-curve/dango/types";
import type { AnyCoin } from "@left-curve/store/types";

export type Notifications = {
  transfer: {
    createdAt: string;
    amount: string;
    coin: AnyCoin;
    fromAddress: Address;
    toAddress: Address;
    type: "received" | "sent";
  };
};

export type Notification<key extends keyof Notifications = keyof Notifications> = {
  id: UID;
  type: string;
  data: Notifications[key];
  blockHeight: number;
  isHidden?: boolean;
  createdAt: string;
};

type UseNotificationsParameters = {
  limit?: number;
  page?: number;
};

export function useNotifications(parameters: UseNotificationsParameters = {}) {
  const { limit = 5, page = 1 } = parameters;

  const { username = "", accounts, account } = useAccount();
  const { coins, subscriptions } = useConfig();

  const [allNotifications, setAllNotifications] = useStorage<Record<Username, Notification[]>>(
    "app.notifications",
    {
      enabled: Boolean(username),
      initialValue: {},
      version: 0.2,
      migrations: {
        0.1: (notifications: Notification[]) => ({ [username]: notifications }),
      },
    },
  );

  const userNotification = useMemo(
    () => (allNotifications[username] || []).filter((n) => !n.isHidden),
    [allNotifications, username],
  );

  const deleteNotification = useCallback(
    (id: UID) => {
      setAllNotifications((notifications) => {
        const previousUserNotification = notifications[username] || [];
        const newNotifications = [...previousUserNotification];
        const notificationIndex = newNotifications.findIndex((n) => n.id === id);
        if (notificationIndex === -1) return notifications;
        newNotifications[notificationIndex].isHidden = true;
        return {
          ...notifications,
          [username]: newNotifications,
        };
      });
    },
    [username],
  );

  const totalNotifications = userNotification.length;
  const hasNotifications = totalNotifications > 0;

  const notifications: Record<string, Notification[]> = useMemo(() => {
    const current = (page - 1) * limit;
    return [...userNotification]
      .reverse()
      .slice(current, current + limit)
      .sort((a, b) => +b.createdAt - +a.createdAt)
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
  }, [userNotification, limit, page]);

  const startNotifications = useCallback(() => {
    if (!account) return;

    const _lastKnownBlockHeight = userNotification.reduce(
      (max, notification) => Math.max(max, notification.blockHeight),
      0,
    );

    const unsubscribe = subscriptions.subscribe("transfer", {
      params: { address: account.address },
      listener: (transfer) => {
        const { fromAddress, toAddress, amount, createdAt, blockHeight } = transfer;
        const coin = coins[transfer.denom];
        const isSent = accounts?.some((a) => a.address === fromAddress);

        const notification = {
          amount,
          createdAt,
          fromAddress,
          toAddress,
          type: isSent ? "sent" : "received",
          coin,
        } as const;

        setAllNotifications((prev) => {
          const previousUserNotification = prev[username] || [];
          return {
            ...prev,
            [username]: [
              ...previousUserNotification,
              { id: uid(), type: "transfer", data: notification, blockHeight, createdAt },
            ],
          };
        });
      },
    });

    return unsubscribe;
  }, [username, accounts, account]);

  return {
    startNotifications,
    deleteNotification,
    notifications,
    hasNotifications,
    totalNotifications,
  };
}
