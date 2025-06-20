import { Toast } from "@left-curve/applets-kit";
import { RootModal } from "./components/modals/RootModal";

import { createToaster } from "@left-curve/applets-kit";
import { DangoStoreProvider } from "@left-curve/store";
import { captureException } from "@sentry/react";
import { QueryCache, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "~/store";

import { AppProvider } from "./app.provider";
import { AppRouter } from "./app.router";

import type React from "react";

import "../public/global.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/normal/800.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/normal/700.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/normal/500.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/normal/400.css";

import "@left-curve/ui-config/fonts/ABCDiatypeRounded/mono/600.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/mono/500.css";

import "@left-curve/ui-config/fonts/Exposure/italic/400.css";
import "@left-curve/ui-config/fonts/Exposure/italic/700.css";

const [Toaster, toast] = createToaster((props) => <Toast {...props} />);

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      if (query.meta?.errorToast) {
        toast.error(query.meta.errorToast);
      }
    },
  }),
  defaultOptions: {
    mutations: {
      onError: (error: unknown) => {
        if (!error) return;
        if (typeof error === "object" && ("code" in error || !(error instanceof Error))) return;
        if (typeof error === "string" && error.includes("reject")) return;

        const errorMessage = error instanceof Error ? error.message : error;
        captureException(errorMessage);
      },
    },
    queries: {
      refetchOnWindowFocus: false,
      retry: 0,
    },
  },
});

export const App: React.FC = () => {
  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <AppProvider toast={toast}>
          <AppRouter />
          <Toaster position="bottom-center" containerStyle={{ zIndex: 99999999 }} />
          <RootModal />
        </AppProvider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
