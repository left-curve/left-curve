import type { FormatNumberOptions } from "@left-curve/dango/utils";
import { createEventBus, useAccount, useConfig, useStorage } from "@left-curve/store";
import * as Sentry from "@sentry/react";
import { type Client as GraphqlSubscriptionClient, createClient } from "graphql-ws";
import { type PropsWithChildren, createContext, useCallback, useEffect, useState } from "react";

import { router } from "./app.router";

import type { AnyCoin } from "@left-curve/store/types";

export type EventBusMap = {
  submit_tx: { isSubmitted: boolean };
  transfer: {
    amount: number;
    coin: AnyCoin;
    fromAddress: string;
    toAddress: string;
    type: "received" | "sent";
  };
};

export type Notifications<key extends keyof EventBusMap = keyof EventBusMap> = {
  createdAt: number;
  type: string;
  data: EventBusMap[key];
};

export type Subscription = {
  transfers: {
    amount: number;
    denom: string;
    fromAddress: string;
    toAddress: string;
    blockHeight: number;
  };
};

export const eventBus = createEventBus<EventBusMap>();

type AppState = {
  router: typeof router;
  eventBus: typeof eventBus;
  notifications: { type: string; data: any; createdAt: number }[];
  isSidebarVisible: boolean;
  setSidebarVisibility: (visibility: boolean) => void;
  isNotificationMenuVisible: boolean;
  setNotificationMenuVisibility: (visibility: boolean) => void;
  isSearchBarVisible: boolean;
  setSearchBarVisibility: (visibility: boolean) => void;
  isQuestBannerVisible: boolean;
  setQuestBannerVisibility: (visibility: boolean) => void;
  showModal: (modalName: string, props?: Record<string, unknown>) => void;
  hideModal: () => void;
  modal: { modal: string | undefined; props: Record<string, unknown> };
  changeSettings: (settings: Partial<AppState["settings"]>) => void;
  settings: {
    showWelcome: boolean;
    isFirstVisit: boolean;
    useSessionKey: boolean;
    formatNumberOptions: FormatNumberOptions;
  };
};

export const AppContext = createContext<AppState | null>(null);

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { chain, coins: chainsCoins } = useConfig();
  const coins = chainsCoins[chain.id];
  // Global component state
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isNotificationMenuVisible, setNotificationMenuVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [isQuestBannerVisible, setQuestBannerVisibility] = useState(true);

  // App settings
  const [settings, setSettings] = useStorage<AppState["settings"]>("app.settings", {
    version: 1.1,
    initialValue: {
      showWelcome: true,
      isFirstVisit: true,
      useSessionKey: true,
      formatNumberOptions: {
        mask: 1,
        language: "en-US",
        maxFractionDigits: 2,
        minFractionDigits: 2,
        notation: "standard",
      },
    },
  });

  // App notifications
  const [notifications, setNotifications] = useStorage<
    { type: string; data: unknown; createdAt: number }[]
  >("app.notifications", { initialValue: [], version: 0.1 });
  const pushNotification = useCallback(
    (notification: { type: string; data: unknown; createdAt: number }) => {
      setNotifications((prev) => [...prev, notification]);
    },
    [notifications],
  );

  const changeSettings = useCallback(
    (s: Partial<AppState["settings"]>) => setSettings((prev) => ({ ...prev, ...s })),
    [],
  );

  // Modal State
  const [modal, setModal] = useState<{
    modal: string | undefined;
    props: Record<string, unknown>;
  }>({ modal: undefined, props: {} });
  const hideModal = useCallback(() => setModal({ modal: undefined, props: {} }), []);
  const showModal = useCallback((modal: string, props = {}) => setModal({ modal, props }), []);

  // Track user errors
  const { username, connector, account } = useAccount();
  useEffect(() => {
    if (!username) Sentry.setUser(null);
    else {
      Sentry.setUser({ username });
      Sentry.setContext("connector", {
        id: connector?.id,
        name: connector?.name,
        type: connector?.type,
      });
    }
  }, [username]);

  // Track notifications
  useEffect(() => {
    if (!username) return;
    let client: GraphqlSubscriptionClient | undefined;
    (async () => {
      client = createClient({ url: chain.urls.indexer });
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
          } as EventBusMap["transfer"];

          eventBus.publish("transfer", notification);
          pushNotification({
            type: "transfer",
            data: notification,
            createdAt: Date.now(),
          });
        }
      }
    })();
    return () => {
      if (client) client.dispose();
    };
  }, [username]);

  return (
    <AppContext.Provider
      value={{
        router,
        eventBus,
        notifications,
        isSidebarVisible,
        setSidebarVisibility,
        isNotificationMenuVisible,
        setNotificationMenuVisibility,
        isSearchBarVisible,
        setSearchBarVisibility,
        isQuestBannerVisible,
        setQuestBannerVisibility,
        showModal,
        hideModal,
        modal,
        settings,
        changeSettings,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};
