import { useUsernames } from "@left-curve/applets-kit";
import { useAccount, useAppConfig, useSessionKey, useStorage } from "@left-curve/store";
import * as Sentry from "@sentry/react";
import { type PropsWithChildren, createContext, useCallback, useEffect, useState } from "react";

import { router } from "./app.router";
import { Modals } from "./components/modals/RootModal";

import { useNotifications } from "./hooks/useNotifications";

import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { notifier as notifierType } from "./hooks/useNotifications";

type AppState = {
  router: typeof router;
  config: ReturnType<typeof useAppConfig>;
  notifier: typeof notifierType;
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

  // App notifications
  const { notifier, subscribe } = useNotifications();
  useEffect(() => {
    if (!account) return;
    const unsubscribe = subscribe(account);
    return () => unsubscribe();
  }, [account]);

  // Track session key expiration
  const { session } = useSessionKey();
  const { usernames } = useUsernames();
  useEffect(() => {
    const intervalId = setInterval(() => {
      if (
        (!session || Date.now() > Number(session.sessionInfo.expireAt)) &&
        usernames.length &&
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
  }, [session, modal, settings.useSessionKey, connector]);

  return (
    <AppContext.Provider
      value={{
        router,
        config,
        notifier,
        isSidebarVisible,
        setSidebarVisibility,
        isNotificationMenuVisible,
        setNotificationMenuVisibility,
        isSearchBarVisible,
        setSearchBarVisibility,
        isQuestBannerVisible: false,
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
