import { useAppConfig, useConfig, useStorage } from "@left-curve/store";
import { useCallback, useState } from "react";
import { createContext } from "../utils/context";

import type { PropsWithChildren } from "react";
import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { ToastController } from "../types/toast";

type AppState = {
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
    chart: "tradingview" | "chartiq";
    showWelcome: boolean;
    isFirstVisit: boolean;
    useSessionKey: boolean;
    formatNumberOptions: FormatNumberOptions;
  };
};

export const [AppContextProvider, useApp] = createContext<AppState>();

type AppProviderProps = {
  toast: ToastController;
};

export const AppProvider: React.FC<PropsWithChildren<AppProviderProps>> = ({ children, toast }) => {
  // Global component state
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isNotificationMenuVisible, setNotificationMenuVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [isTradeBarVisible, setTradeBarVisibility] = useState(false);
  const [isQuestBannerVisible, setQuestBannerVisibility] = useState(false);

  // App settings
  const [settings, setSettings] = useStorage<AppState["settings"]>("app.settings", {
    version: 1.4,
    initialValue: {
      chart: "tradingview",
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
    sync: true,
    migrations: {
      1.2: (state: AppState["settings"]) => {
        state.showWelcome = true;
        return state;
      },
      1.3: (state: AppState["settings"]) => {
        state.chart = "tradingview";
        return state;
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

  return (
    <AppContextProvider
      value={{
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
    </AppContextProvider>
  );
};
