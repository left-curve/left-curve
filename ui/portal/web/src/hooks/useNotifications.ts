import { useAccount, useConfig, useStorage } from "@left-curve/store";
import { useCallback, useMemo } from "react";

import { uid } from "@left-curve/dango/utils";
import { format, isToday } from "date-fns";

import type { AccountTypes, Address, Hex, UID, Username } from "@left-curve/dango/types";
import type { AnyCoin } from "@left-curve/store/types";

export type Notifications = {
  transfer: {
    createdAt: string;
    amount: string;
    coin: AnyCoin;
    fromAddress: Address;
    toAddress: Address;
    txHash: Hex;
    type: "received" | "sent";
  };
  account: {
    address: Address;
    accountType: AccountTypes;
    accountIndex: number;
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

  const addNotification = useCallback(
    (notification: Notification) => {
      setAllNotifications((notifications) => {
        const previousUserNotification = notifications[username] || [];
        return {
          ...notifications,
          [username]: [...previousUserNotification, notification],
        };
      });
    },
    [username],
  );

  const deleteNotification = useCallback(
    (id: UID) => {
      setAllNotifications((notifications) => {
        const previousUserNotification = notifications[username] || [];
        const notificationIndex = previousUserNotification.findIndex((n) => n.id === id);
        if (notificationIndex === -1) return notifications;
        previousUserNotification[notificationIndex] = {
          ...previousUserNotification[notificationIndex],
          isHidden: true,
        };
        return {
          ...notifications,
          [username]: previousUserNotification,
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
    if (!account || !username) return;

    const lastKnownBlockHeight = userNotification.reduce(
      (max, notification) => Math.max(max, notification.blockHeight),
      0,
    );

    const sinceBlockHeight = lastKnownBlockHeight + 1;

    const unsubscribeAccount = subscriptions.subscribe("account", {
      params: { username },
      listener: ({ accounts }) => {
        for (const account of accounts) {
          const { address, accountType, accountIndex, createdAt, createdBlockHeight } = account;

          const notification = {
            address,
            accountType,
            accountIndex,
          };

          addNotification({
            id: uid(),
            type: "account",
            data: notification,
            blockHeight: createdBlockHeight,
            createdAt,
          });
        }
      },
    });

    const unsubscribeTransfer = subscriptions.subscribe("transfer", {
      params: { username },
      listener: ({ transfers }) => {
        for (const transfer of transfers) {
          const { id, fromAddress, toAddress, amount, denom, blockHeight, createdAt, txHash } =
            transfer;

          const coin = coins[denom];

          const notification = {
            createdAt,
            amount,
            txHash,
            coin,
            fromAddress,
            toAddress,
            type: fromAddress === account.address ? "sent" : "received",
          } as const;

          addNotification({
            id,
            type: "transfer",
            data: notification,
            blockHeight,
            createdAt,
          });
        }
      },
    });

    return () => {
      unsubscribeTransfer();
      unsubscribeAccount();
    };
  }, [addNotification, userNotification, username, accounts, account]);

  return {
    startNotifications,
    deleteNotification,
    notifications,
    hasNotifications,
    totalNotifications,
  };
}
