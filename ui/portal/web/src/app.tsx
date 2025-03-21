import type React from "react";
import { Toaster } from "react-hot-toast";
import { Modal } from "./components/foundation/Modal";

import { DangoStoreProvider } from "@left-curve/store";
import { QueryCache, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "../store.config";

import { AppProvider } from "./app.provider";
import { AppRouter } from "./app.router";

import "../public/global.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/index.css";
import "@left-curve/ui-config/fonts/Exposure/index.css";
import { toast } from "./components/foundation/Toast";

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      console.error(error);
      if (query.meta?.errorToast) {
        toast.error(query.meta.errorToast);
      }
    },
  }),
  defaultOptions: {
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
        <AppProvider>
          <AppRouter />
          <Toaster position="bottom-center" />
          <Modal />
        </AppProvider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
