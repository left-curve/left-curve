import { type UseStorageOptions, useConfig, useStorage } from "@left-curve/store";
import { useCallback, useEffect, useState } from "react";
import { createStore } from "zustand/vanilla";
import { createContext } from "../utils/context";

import type { PropsWithChildren } from "react";
import type { FormatNumberOptions } from "@left-curve/utils";
import type { ToastController } from "../types/toast";
import type { StoreApi } from "zustand/vanilla";

export type AppState = {
  toast: ToastController;
  subscriptions: ReturnType<typeof useConfig>["subscriptions"];
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

export type AppStore = StoreApi<AppState>;

export const [AppContextProvider, useAppStore] = createContext<AppStore>({
  name: "AppContext",
});

export type AppProviderProps = {
  toast: ToastController;
  navigate: AppState["navigate"];
};

const appSettingsStorageOptions = {
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
        language: state.formatNumberOptions.language,
      };
      return state;
    },
  },
} satisfies UseStorageOptions<AppState["settings"]>;

function createAppStore({
  changeSettings,
  navigate,
  settings,
  subscriptions,
  toast,
}: Pick<AppState, "changeSettings" | "navigate" | "settings" | "subscriptions" | "toast">) {
  return createStore<AppState>()((set) => ({
    navigate,
    subscriptions,
    isSidebarVisible: false,
    setSidebarVisibility: (isSidebarVisible) => set({ isSidebarVisible }),
    isSearchBarVisible: false,
    setSearchBarVisibility: (isSearchBarVisible) => set({ isSearchBarVisible }),
    isTradeBarVisible: false,
    setTradeBarVisibility: (isTradeBarVisible) => set({ isTradeBarVisible }),
    isQuestBannerVisible: true,
    setQuestBannerVisibility: (isQuestBannerVisible) => set({ isQuestBannerVisible }),
    showModal: (modal, props = {}) => set({ modal: { modal, props } }),
    hideModal: () => set({ modal: { modal: undefined, props: {} } }),
    modal: { modal: undefined, props: {} },
    toast,
    settings,
    changeSettings,
  }));
}

export const AppProvider: React.FC<PropsWithChildren<AppProviderProps>> = ({
  children,
  toast,
  navigate,
}) => {
  const [settings, setSettings] = useStorage<AppState["settings"]>(
    "app.settings",
    appSettingsStorageOptions,
  );
  const { subscriptions } = useConfig();

  const changeSettings = useCallback(
    (nextSettings: Partial<AppState["settings"]>) =>
      setSettings((previous) => ({ ...previous, ...nextSettings })),
    [setSettings],
  );

  const [store] = useState(() =>
    createAppStore({
      changeSettings,
      navigate,
      settings,
      subscriptions,
      toast,
    }),
  );

  useEffect(() => {
    store.setState({
      changeSettings,
      navigate,
      subscriptions,
      toast,
      settings,
    });
  }, [store, changeSettings, navigate, subscriptions, toast, settings]);

  return <AppContextProvider value={store}>{children}</AppContextProvider>;
};
