import { useAccount, useAppConfig, useConfig, useSessionKey, useStorage } from "@left-curve/store";
import { type PropsWithChildren, createContext, useCallback, useEffect, useState } from "react";
import { useNotifications } from "./hooks/useNotifications";
import { useTheme } from "@left-curve/applets-kit";

import * as Sentry from "@sentry/react";
import { router } from "./app.router";
import { Modals } from "./components/modals/RootModal";

import type { ToastController } from "@left-curve/applets-kit";
import type { FormatNumberOptions } from "@left-curve/dango/utils";

type AppState = {
  router: typeof router;
  toast: ToastController;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  config: ReturnType<typeof useAppConfig>;
  isSidebarVisible: boolean;
  setSidebarVisibility: (visibility: boolean) => void;
  isNotificationMenuVisible: boolean;
  setNotificationMenuVisibility: (visibility: boolean) => void;
  isSearchBarVisible: boolean;
  setSearchBarVisibility: (visibility: boolean) => void;
  isTradeBarVisible: boolean;
  setTradeBarVisibility: (visibility: boolean) => void;
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

type AppProviderProps = {
  toast: ToastController;
};

export const AppProvider: React.FC<PropsWithChildren<AppProviderProps>> = ({ children, toast }) => {
  // Global component state
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isNotificationMenuVisible, setNotificationMenuVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [isTradeBarVisible, setTradeBarVisibility] = useState(false);
  const [isQuestBannerVisible, setQuestBannerVisibility] = useState(true);

  // Initialize theme
  const theme = useTheme();

  // App settings
  const [settings, setSettings] = useStorage<AppState["settings"]>("app.settings", {
    version: 1.2,
    initialValue: {
      showWelcome: true,
      isFirstVisit: true,
      useSessionKey: true,
      formatNumberOptions: {
        mask: 1,
        language: "en-US",
        maxFractionDigits: 4,
        minFractionDigits: 0,
        notation: "standard",
      },
    },
  });

  // App Config
  const { subscriptions } = useConfig();
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

  // Initialize notifications
  const { startNotifications } = useNotifications();
  useEffect(() => {
    const stopNotifications = startNotifications();
    return stopNotifications;
  }, [account]);

  // Track session key expiration
  const { session } = useSessionKey();
  useEffect(() => {
    const intervalId = setInterval(() => {
      if (
        (!session || Date.now() > Number(session.sessionInfo.expireAt)) &&
        account &&
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
        subscriptions,
        isSidebarVisible,
        setSidebarVisibility,
        isNotificationMenuVisible,
        setNotificationMenuVisibility,
        isSearchBarVisible,
        setSearchBarVisibility,
        isTradeBarVisible,
        setTradeBarVisibility,
        isQuestBannerVisible,
        setQuestBannerVisibility,
        showModal,
        hideModal,
        modal,
        toast,
        settings,
        changeSettings,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};
