import { useAppConfig, useConfig, useStorage } from "@left-curve/store";
import { useCallback, useState } from "react";
import { createContext } from "../utils/context";

import type { PropsWithChildren } from "react";
import type { FormatNumberOptions } from "@left-curve/dango/utils";
import type { ToastController } from "../types/toast";

export type AppState = {
  toast: ToastController;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
  config: ReturnType<typeof useAppConfig>;
  isSidebarVisible: boolean;
  setSidebarVisibility: (visibility: boolean) => void;
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
    chart: "tradingview";
    timeFormat: "hh:mm a" | "hh:mm aa" | "HH:mm";
    dateFormat: "MM/dd/yyyy" | "dd/MM/yyyy" | "yyyy/MM/dd";
    timeZone: "local" | "utc";
    showWelcome: boolean;
    isFirstVisit: boolean;
    useSessionKey: boolean;
    formatNumberOptions: FormatNumberOptions;
  };
  navigate: (to: string, options?: { replace?: boolean }) => void;
};

export const [AppContextProvider, useApp] = createContext<AppState>();

export type AppProviderProps = {
  toast: ToastController;
  navigate: AppState["navigate"];
};

export const AppProvider: React.FC<PropsWithChildren<AppProviderProps>> = ({
  children,
  toast,
  navigate,
}) => {
  // Global component state
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [isTradeBarVisible, setTradeBarVisibility] = useState(false);
  const [isQuestBannerVisible, setQuestBannerVisibility] = useState(true);

  // App settings
  const [settings, setSettings] = useStorage<AppState["settings"]>("app.settings", {
    version: 1.8,
    initialValue: {
      chart: "tradingview",
      showWelcome: true,
      isFirstVisit: true,
      useSessionKey: true,
      timeFormat: "hh:mm a",
      dateFormat: "MM/dd/yyyy",
      timeZone: "local",
      formatNumberOptions: {
        mask: 1,
        maximumTotalDigits: 8,
        language: "en-US",
      },
    },
    sync: true,
    migrations: {
      "*": (state: AppState["settings"]) => {
        state.showWelcome = Object.hasOwn(state, "showWelcome") ? state.showWelcome : true;
        state.chart = "tradingview";
        state.timeFormat = state.timeFormat || "hh:mm a";
        state.dateFormat = state.dateFormat || "MM/dd/yyyy";
        state.timeZone = state.timeZone || "local";
        state.formatNumberOptions = {
          mask: state.formatNumberOptions.mask,
          maximumTotalDigits: 8,
          language: state.formatNumberOptions.language,
        };
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
        navigate,
        subscriptions,
        isSidebarVisible,
        setSidebarVisibility,
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
