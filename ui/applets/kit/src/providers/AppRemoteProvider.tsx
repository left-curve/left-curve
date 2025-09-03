import {
  type AppState,
  createContext,
  type ToastController,
  type ToastMsg,
  type ToastOptions,
} from "@left-curve/foundation";
import { requestRemote, useAppConfig, useConfig, type WindowDangoStore } from "@left-curve/store";
import { useTheme } from "../hooks/useTheme";

import type { PropsWithChildren } from "react";
import type React from "react";

export interface WindowDangoRemoteApp extends WindowDangoStore {
  dango: WindowDangoStore["dango"] & {
    settings: AppState["settings"];
  };
}

declare let window: WindowDangoRemoteApp;

const navigate = (to: string, options?: { replace?: boolean }) => {
  requestRemote("navigate", to, options);
};

const hideModal = () => requestRemote("hideModal");

const showModal = (modalName: string, props?: Record<string, unknown>) => {
  requestRemote("showModal", { modalName, props });
};

const toast = {
  success: (toastMsg?: ToastMsg, options?: ToastOptions) => {
    requestRemote<string>("toast", "success", toastMsg, options);
  },
  error: (toastMsg?: ToastMsg, options?: ToastOptions) => {
    requestRemote<string>("toast", "error", toastMsg, options);
  },
} as ToastController;

const [RemoteContextProvider, useRemoteApp] = createContext<AppState>();

export { useRemoteApp };

export const AppRemoteProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { subscriptions } = useConfig();
  const config = useAppConfig();
  const _theme = useTheme();

  return (
    <RemoteContextProvider
      value={
        {
          subscriptions,
          config,
          toast,
          settings: window.dango.settings,
          navigate,
          showModal,
          hideModal,
        } as unknown as AppState
      }
    >
      {children}
    </RemoteContextProvider>
  );
};
