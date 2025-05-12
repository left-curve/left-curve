import type { FormatNumberOptions } from "@left-curve/dango/utils";
import {
  createEventBus,
  useAccount,
  useAppConfig,
  useConfig,
  useSessionKey,
  useStorage,
} from "@left-curve/store";
import * as Sentry from "@sentry/react";
import { type Client as GraphqlSubscriptionClient, createClient } from "graphql-ws";
import { type PropsWithChildren, createContext, useCallback, useEffect, useState } from "react";

import { GRAPHQL_URI } from "../store.config";
import { router } from "./app.router";
import { Modals } from "./components/modals/RootModal";

import type { AnyCoin } from "@left-curve/store/types";

export type NotificationsMap = {
  submit_tx:
    | { isSubmitting: true; txResult?: never }
    | { isSubmitting: false; txResult: { hasSucceeded: boolean; message: string } };
  transfer: {
    amount: number;
    coin: AnyCoin;
    fromAddress: string;
    toAddress: string;
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
    fromAddress: string;
    toAddress: string;
    blockHeight: number;
  };
};

export const notifier = createEventBus<NotificationsMap>();

type AppState = {
  router: typeof router;
  config: ReturnType<typeof useAppConfig>;
  notifier: typeof notifier;
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
  const { coins } = useConfig();
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

  // App Config
  const config = useAppConfig();

  // App notifications
  const [notifications, setNotifications] = useStorage<
    { type: string; data: unknown; createdAt: number }[]
  >("app.notifications", { initialValue: [], version: 0.1 });
  const pushNotification = useCallback(
    (notification: { type: string; data: unknown; createdAt: number }) => {
      setNotifications((prev) => [...prev, notification]);
    },
    [],
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

  // Track session key expiration
  const { session } = useSessionKey();
  useEffect(() => {
    const intervalId = setInterval(() => {
      if (
        (!session || Date.now() > Number(session.sessionInfo.expireAt)) &&
        settings.useSessionKey &&
        connector &&
        connector.type !== "session"
      ) {
        if (modal.modal !== Modals.RenewSession) {
          showModal(Modals.RenewSession);
        }
      }
    }, 1000);
    return () => {
      clearInterval(intervalId);
    };
  }, [session, modal]);

  // Track notifications
  useEffect(() => {
    if (!username) return;
    let client: GraphqlSubscriptionClient | undefined;
    (async () => {
      client = createClient({ url: GRAPHQL_URI });
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
        config,
        notifier,
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
