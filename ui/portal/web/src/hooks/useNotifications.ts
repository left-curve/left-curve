import { useAccount, useConfig, usePublicClient, useStorage } from "@left-curve/store";
import { useCallback, useMemo } from "react";

import { uid } from "@left-curve/dango/utils";
import { format, isToday } from "date-fns";

import type {
  AccountTypes,
  Address,
  Coins,
  Hex,
  OrderCanceledEvent,
  OrderCreatedEvent,
  OrderFilledEvent,
  OrderResponse,
  OrderTypes,
  UID,
  Username,
} from "@left-curve/dango/types";
import { useQueryClient } from "@tanstack/react-query";

export type Notifications = {
  transfer: {
    coins: Coins;
    fromAddress: Address;
    toAddress: Address;
    type: "received" | "sent";
  };
  account: {
    address: Address;
    accountType: AccountTypes;
    accountIndex: number;
  };
  orderCreated: OrderCreatedEvent;
  orderCanceled: OrderResponse & { kind: OrderTypes };
  orderFilled: OrderFilledEvent;
};

export type Notification<key extends keyof Notifications = keyof Notifications> = {
  id: UID;
  type: key;
  data: Notifications[key];
  blockHeight: number;
  seen?: boolean;
  isHidden?: boolean;
  txHash?: Hex;
  createdAt: string;
};

type UseNotificationsParameters = {
  limit?: number;
  page?: number;
};

export function useNotifications(parameters: UseNotificationsParameters = {}) {
  const { limit = 5, page = 1 } = parameters;
  const queryClient = useQueryClient();
  const publicClient = usePublicClient();

  const { username = "", accounts, account } = useAccount();
  const { subscriptions } = useConfig();
  const userAddresses = useMemo(() => (accounts ? accounts.map((a) => a.address) : []), [accounts]);

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
            seen: false,
            blockHeight: createdBlockHeight,
            createdAt,
          });
        }
      },
    });

    const addresses = accounts?.map(({ address }) => address) || [];
    const unsubscribeEvents = subscriptions.subscribe("eventsByAddresses", {
      params: { addresses },
      listener: async (events) => {
        for (const event of events) {
          const { data: eventData, blockHeight, createdAt, transaction } = event;
          if (!("contract_event" in eventData)) continue;
          const { type, data } = eventData.contract_event;

          const notification = await (async () => {
            switch (type) {
              case "sent":
              case "received": {
                const isSent = type === "sent";
                const { to, from, user, coins } = data as {
                  to?: Address;
                  from?: Address;
                  user: Address;
                  coins: Record<string, string>;
                };

                if (isSent && !userAddresses.includes(user as Address)) return;
                if (!isSent && !userAddresses.includes(user as Address)) return;
                if (!Object.keys(coins).length) return;

                const notification = {
                  coins,
                  fromAddress: from || user,
                  toAddress: to || user,
                  type,
                };

                return { data: notification, type: "transfer" as const };
              }
              case "order_filled": {
                return { data: data as OrderFilledEvent, type: "orderFilled" as const };
              }
              case "order_created": {
                return { data: data as OrderCreatedEvent, type: "orderCreated" as const };
              }
              case "order_canceled": {
                const { id, kind } = data as OrderCanceledEvent;
                const notification = await queryClient.fetchQuery({
                  queryKey: ["order", id],
                  queryFn: () => publicClient.getOrder({ orderId: id, height: blockHeight - 2 }),
                });
                console.log(notification);

                return { data: { ...notification, kind }, type: "orderCanceled" as const };
              }
            }
          })();

          if (!notification) continue;

          addNotification({
            id: uid(),
            data: notification.data,
            type: notification.type,
            txHash: transaction?.hash,
            seen: false,
            blockHeight,
            createdAt,
          });
        }
      },
    });

    return () => {
      unsubscribeEvents();
      unsubscribeAccount();
    };
  }, [addNotification, userNotification, username, accounts, account, userAddresses]);

  return {
    startNotifications,
    deleteNotification,
    notifications,
    hasNotifications,
    totalNotifications,
  };
}
