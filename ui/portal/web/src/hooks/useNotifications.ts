import { createEventBus, useAccount, useConfig, useStorage } from "@left-curve/store";
import { format, isToday } from "date-fns";
import { type Client as GraphqlSubscriptionClient, createClient } from "graphql-ws";
import { useCallback, useMemo, useRef } from "react";

import type { Account, Address, Username } from "@left-curve/dango/types";
import type { AnyCoin } from "@left-curve/store/types";

export type NotificationsMap = {
  submit_tx:
    | { isSubmitting: true; txResult?: never }
    | { isSubmitting: false; txResult: { hasSucceeded: boolean; message: string } };
  transfer: {
    amount: number;
    coin: AnyCoin;
    fromAddress: Address;
    toAddress: Address;
    type: "received" | "sent";
  };
};

export type Notifications<key extends keyof NotificationsMap = keyof NotificationsMap> = {
  createdAt: number;
  type: string;
  data: NotificationsMap[key];
};

export type Subscription = {
  transfers: {
    amount: number;
    denom: string;
    fromAddress: Address;
    toAddress: Address;
    blockHeight: number;
  };
};

type UseNotificationsParameters = {
  limit?: number;
  page?: number;
};

export const notifier = createEventBus<NotificationsMap>();

export function useNotifications(parameters: UseNotificationsParameters = {}) {
  const { limit = 5, page = 1 } = parameters;

  const graphqlClient = useRef<GraphqlSubscriptionClient>();
  const { username = "" } = useAccount();
  const { coins, chain } = useConfig();

  const [allNotifications, setAllNotifications] = useStorage<Record<Username, Notifications[]>>(
    "app.notifications",
    {
      enabled: Boolean(username),
      initialValue: {},
      version: 0.2,
      migrations: {
        0.1: (notifications: Notifications[]) => ({ [username]: notifications }),
      },
    },
  );

  const userNotification = allNotifications[username] || [];

  const totalNotifications = userNotification.length;
  const hasNotifications = totalNotifications > 0;

  const notifications: Record<string, Notifications[]> = useMemo(() => {
    const current = (page - 1) * limit;
    return [...userNotification]
      .reverse()
      .slice(current, current + limit)
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
  }, [userNotification, limit, page]);

  const subscribe = useCallback(
    (account: Account) => {
      const client = graphqlClient.current
        ? graphqlClient.current
        : createClient({ url: chain.urls.indexer });

      (async () => {
        const subscription = client.iterate({
          query: `subscription($address: String) {
              sentTransfers: transfers(fromAddress: $address) {
                fromAddress
                toAddress
                blockHeight
                amount
                denom
              }
              receivedTransfers: transfers(toAddress: $address) {
                fromAddress
                toAddress
                blockHeight
                amount
                denom
              }
            }`,
          variables: { address: account?.address },
        });
        for await (const { data } of subscription) {
          if (!data) continue;
          if ("receivedTransfers" in data || "sentTransfers" in data) {
            const isSent = "sentTransfers" in data;

            const [transfer] = data[
              isSent ? "sentTransfers" : "receivedTransfers"
            ] as Subscription["transfers"][];
            if (!transfer) continue;
            const coin = coins[transfer.denom];
            const notification = {
              ...transfer,
              type: isSent ? "sent" : "received",
              coin,
            } as NotificationsMap["transfer"];

            notifier.publish("transfer", notification);
            setAllNotifications((prev) => {
              const previousUserNotification = prev[username] || [];
              return {
                ...prev,
                [username]: [
                  ...previousUserNotification,
                  { type: "transfer", data: notification, createdAt: Date.now() },
                ],
              };
            });
          }
        }
      })();
      return () => {
        if (client) client.dispose();
      };
    },
    [username],
  );

  return {
    notifier,
    subscribe,
    notifications,
    hasNotifications,
    totalNotifications,
  };
}
