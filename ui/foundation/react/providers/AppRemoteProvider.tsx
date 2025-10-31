import { createContext } from "../utils/context";
import { requestRemote, useAppConfig, useConfig, type WindowDangoStore } from "@left-curve/store";

import type { PropsWithChildren } from "react";
import type React from "react";
import type { AppState } from "./AppProvider";
import type { ToastController } from "../types/toast";

export interface WindowDangoRemoteApp extends WindowDangoStore {
  dango: WindowDangoStore["dango"] & {
    settings: AppState["settings"];
  };
}

declare let window: WindowDangoRemoteApp;

const navigate = (to: string, options?: { replace?: boolean }) => {
  requestRemote("navigate", to, options);
};

const hideModal = () => {
  requestRemote("hideModal");
};

const showModal = (modalName: string, props?: Record<string, unknown>) => {
  requestRemote("showModal", { modalName, props });
};

const toast = {
  success: (toastMsg, options) => {
    requestRemote<string>("toast", "success", toastMsg, options);
  },
  error: (toastMsg, options) => {
    requestRemote<string>("toast", "error", toastMsg, options);
  },
} as ToastController;

const [RemoteContextProvider, useRemoteApp] = createContext<AppState>();

export { useRemoteApp };

export const AppRemoteProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { subscriptions } = useConfig();
  const config = useAppConfig();

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
        } as AppState
      }
    >
      {children}
    </RemoteContextProvider>
  );
};
