import {
  type AppState,
  createContext,
  type ToastController,
  type ToastMsg,
  type ToastOptions,
} from "@left-curve/foundation";
import { useAppConfig, useConfig, type WindowDangoStore } from "@left-curve/store";
import { useTheme } from "../hooks/useTheme";

import { serializeJson } from "@left-curve/dango/encoding";

import type { PropsWithChildren } from "react";
import type React from "react";

export interface WindowDangoRemoteApp extends WindowDangoStore {
  dango: WindowDangoStore["dango"] & {
    settings: AppState["settings"];
  };
  ReactNativeWebView: {
    postMessage: (message: string) => void;
  };
}

declare let window: WindowDangoRemoteApp;

const sendMessage = window.ReactNativeWebView?.postMessage;

const showModal = (modalName: string, props?: Record<string, unknown>) => {
  sendMessage(serializeJson({ type: "app.showModal", parameters: { modalName, props } }));
};

const hideModal = () => {
  sendMessage(serializeJson({ type: "app.hideModal" }));
};

const toast = {
  success: (toastMsg?: ToastMsg, options?: ToastOptions) => {
    sendMessage(serializeJson({ type: "toast.success", parameters: { toastMsg, options } }));
    return "";
  },
  error: (toastMsg?: ToastMsg, options?: ToastOptions) => {
    sendMessage(serializeJson({ type: "toast.error", parameters: { toastMsg, options } }));
    return "";
  },
  loading: (toastMsg?: ToastMsg, options?: ToastOptions) => {
    sendMessage(serializeJson({ type: "toast.loading", parameters: { toastMsg, options } }));
    return "";
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
          showModal,
          hideModal,
        } as AppState
      }
    >
      {children}
    </RemoteContextProvider>
  );
};
