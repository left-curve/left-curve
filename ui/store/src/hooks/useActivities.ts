import { useCallback, useMemo } from "react";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { useStorage } from "./useStorage.js";
import { useQueryClient } from "@tanstack/react-query";

import { snakeToCamel, uid } from "@left-curve/dango/utils";

import type {
  Address,
  Coins,
  Hex,
  OrderCanceledEvent,
  OrderCreatedEvent,
  OrderFilledEvent,
  OrderFilledData,
  LiquidatedData,
  DeleveragedData,
  UID,
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
    accountIndex: number;
  };
  orderCreated: OrderCreatedEvent;
  orderCanceled: OrderCanceledEvent;
  orderFilled: OrderFilledEvent;
  perpOrderFilled: OrderFilledData;
  perpLiquidated: LiquidatedData;
  perpDeleveraged: DeleveragedData;
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
  const { userIndex, accounts, account, refreshUserStatus, userStatus } = useAccount();
  const { refetch: refetchBalances } = useBalances({ address: account?.address });
  const { subscriptions } = useConfig();
  const userAddresses = useMemo(() => (accounts ? accounts.map((a) => a.address) : []), [accounts]);

  const existUserIndex = userIndex !== undefined;

  const [allActivities, setAllActivities] = useStorage<Record<number, ActivityRecord[]>>(
    "app.activities",
    {
      enabled: existUserIndex,
      initialValue: {},
      version: 0.2,
    },
  );

  const userActivities = useMemo(
    () => (existUserIndex ? allActivities[userIndex] || [] : []).filter((n) => !n.isHidden),
    [allActivities, existUserIndex, userIndex],
  );

  const totalActivities = userActivities.length;

  const addActivityRecord = useCallback(
    (activity: ActivityRecord) => {
      if (!existUserIndex) return;
      setAllActivities((activities) => {
        const previousUserActivities = activities[userIndex] || [];
        return {
          ...activities,
          [userIndex]: [...previousUserActivities, activity],
        };
      });
    },
    [userIndex],
  );

  const deleteActivityRecord = useCallback(
    (id: UID) => {
      if (!existUserIndex) return;
      setAllActivities((activities) => {
        const previousUserActivities = activities[userIndex] || [];
        const activityIndex = previousUserActivities.findIndex((n) => n.id === id);
        if (activityIndex === -1) return activities;
        previousUserActivities[activityIndex] = {
          ...previousUserActivities[activityIndex],
          isHidden: true,
        };
        return {
          ...activities,
          [userIndex]: previousUserActivities,
        };
      });
    },
    [userIndex],
  );

  const unseenCount = useMemo(
    () => userActivities.filter((a) => a.seen === false).length,
    [userActivities],
  );

  const markAllSeen = useCallback(() => {
    if (!existUserIndex) return;
    setAllActivities((activities) => {
      const updated = (activities[userIndex] || []).map((a) => ({ ...a, seen: true }));
      return { ...activities, [userIndex]: updated };
    });
  }, [userIndex]);

  const hasActivities = totalActivities > 0;

  const startActivities = useCallback(() => {
    if (!account || !existUserIndex) return;

    const lastKnownBlockHeight = userActivities.reduce(
      (max, activity) => Math.max(max, activity.blockHeight),
      0,
    );

    const sinceBlockHeight = lastKnownBlockHeight + 1;

    const unsubscribeAccount = subscriptions.subscribe("account", {
      params: { userIndex: account.owner },
      listener: ({ accounts }) => {
        for (const account of accounts) {
          const { address, accountIndex, createdAt, createdBlockHeight } = account;

          const activity = {
            address,
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
                if (userStatus !== "active") refreshUserStatus?.();
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
              case "order_filled": {
                queryClient.invalidateQueries({ queryKey: ["ordersByUser", account?.address] });
                queryClient.invalidateQueries({ queryKey: ["tradeHistory", account?.address] });
                refetchBalances();
                const isPerps = "pair_id" in (data as Record<string, unknown>);
                return {
                  data: data as Activities[keyof Activities],
                  type: (isPerps ? "perpOrderFilled" : "orderFilled") as keyof Activities,
                };
              }
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
              case "liquidated": {
                refetchBalances();
                return {
                  data: data as Activities["perpLiquidated"],
                  type: "perpLiquidated" as const,
                };
              }
              case "deleveraged": {
                refetchBalances();
                return {
                  data: data as Activities["perpDeleveraged"],
                  type: "perpDeleveraged" as const,
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
  }, [addActivityRecord, userActivities, userIndex, accounts, account, userAddresses, userStatus]);

  return {
    startActivities,
    deleteActivityRecord,
    userActivities,
    hasActivities,
    totalActivities,
    unseenCount,
    markAllSeen,
  };
}
