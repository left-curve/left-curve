import type { FormatNumberOptions } from "@left-curve/dango/utils";
import { createEventBus, useAccount, useConfig, useStorage } from "@left-curve/store";
import * as Sentry from "@sentry/react";
import { type Client as GraphqlSubscriptionClient, createClient } from "graphql-ws";
import { type PropsWithChildren, createContext, useCallback, useEffect, useState } from "react";

import { router } from "./app.router";

export type EventBusMap = {
  submit_tx: { isSubmitted: boolean };
  transfer: { amount: number; denom: string; from: string; to: string };
};

export const eventBus = createEventBus<EventBusMap>();

type AppState = {
  router: typeof router;
  eventBus: typeof eventBus;
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
  const { chain } = useConfig();
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
  const { username, connector } = useAccount();
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
        query: `subscription {
            transfers {
              fromAddress,
              toAddress,
              amount,
              denom
          }
        }`,
      });
      for await (const { data } of subscription) {
        if (!data) continue;
        if ("transfers" in data) {
          eventBus.publish("transfer", data.transfers as EventBusMap["transfer"]);
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
