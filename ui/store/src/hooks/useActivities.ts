import { useCallback, useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { useStorage } from "./useStorage.js";
import { useQueryClient } from "@tanstack/react-query";

import { snakeToCamel, uid } from "@left-curve/dango/utils";

import type {
  AccountTypes,
  Address,
  Coins,
  Hex,
  OrderCanceledEvent,
  OrderCreatedEvent,
  OrderFilledEvent,
  UID,
  Username,
} from "@left-curve/dango/types";
import { useBalances } from "./useBalances.js";

export type Activities = {
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
  orderCanceled: OrderCanceledEvent;
  orderFilled: OrderFilledEvent;
};

export type ActivityRecord<key extends keyof Activities = keyof Activities> = {
  id: UID;
  type: key;
  data: Activities[key];
  blockHeight: number;
  seen?: boolean;
  isHidden?: boolean;
  txHash?: Hex;
  createdAt: string;
};

export function useActivities() {
  const queryClient = useQueryClient();
  const { username = "", accounts, account } = useAccount();
  const { refetch: refetchBalances } = useBalances({ address: account?.address });
  const { subscriptions } = useConfig();
  const userAddresses = useMemo(() => (accounts ? accounts.map((a) => a.address) : []), [accounts]);

  const [allActivities, setAllActivities] = useStorage<Record<Username, ActivityRecord[]>>(
    "app.activities",
    {
      enabled: Boolean(username),
      initialValue: {},
      version: 0.1,
    },
  );

  const userActivities = useMemo(
    () => (allActivities[username] || []).filter((n) => !n.isHidden),
    [allActivities, username],
  );

  const totalActivities = userActivities.length;

  const addActivityRecord = useCallback(
    (activity: ActivityRecord) => {
      setAllActivities((activities) => {
        const previousUserActivities = activities[username] || [];
        return {
          ...activities,
          [username]: [...previousUserActivities, activity],
        };
      });
    },
    [username],
  );

  const deleteActivityRecord = useCallback(
    (id: UID) => {
      setAllActivities((activities) => {
        const previousUserActivities = activities[username] || [];
        const activityIndex = previousUserActivities.findIndex((n) => n.id === id);
        if (activityIndex === -1) return activities;
        previousUserActivities[activityIndex] = {
          ...previousUserActivities[activityIndex],
          isHidden: true,
        };
        return {
          ...activities,
          [username]: previousUserActivities,
        };
      });
    },
    [username],
  );

  const hasActivities = totalActivities > 0;

  const startActivities = useCallback(() => {
    if (!account || !username) return;

    const lastKnownBlockHeight = userActivities.reduce(
      (max, activity) => Math.max(max, activity.blockHeight),
      0,
    );

    const sinceBlockHeight = lastKnownBlockHeight + 1;

    const unsubscribeAccount = subscriptions.subscribe("account", {
      params: { username },
      listener: ({ accounts }) => {
        for (const account of accounts) {
          const { address, accountType, accountIndex, createdAt, createdBlockHeight } = account;

          const activity = {
            address,
            accountType,
            accountIndex,
          };

          addActivityRecord({
            id: uid(),
            type: "account",
            data: activity,
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
      listener: (events) => {
        for (const event of events) {
          const { data: eventData, blockHeight, createdAt, transaction } = event;
          if (!("contract_event" in eventData)) continue;
          const { type, data } = eventData.contract_event;

          const activity = (() => {
            switch (type) {
              case "sent":
              case "received": {
                refetchBalances();
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

                const details = {
                  coins,
                  fromAddress: from || user,
                  toAddress: to || user,
                  type,
                };

                return { data: details, type: "transfer" as const };
              }
              case "order_filled":
              case "order_created":
              case "order_canceled": {
                queryClient.invalidateQueries({ queryKey: ["ordersByUser", account?.address] });
                queryClient.invalidateQueries({ queryKey: ["tradeHistory", account?.address] });
                refetchBalances();
                return {
                  data: data as Activities[keyof Activities],
                  type: snakeToCamel(type) as keyof Activities,
                };
              }
            }
          })();

          if (!activity) continue;

          addActivityRecord({
            id: uid(),
            data: activity.data,
            type: activity.type,
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
  }, [addActivityRecord, userActivities, username, accounts, account, userAddresses]);

  return {
    startActivities,
    deleteActivityRecord,
    userActivities,
    hasActivities,
    totalActivities,
  };
}
