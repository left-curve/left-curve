import { DangoStoreProvider } from "@left-curve/store";
import { QueryClientProvider } from "@tanstack/react-query";
import { config } from "~/store";

import { AppRouter, router } from "./app.router";
import { AppProvider } from "@left-curve/foundation";
import { Toaster, toast } from "@left-curve/applets-kit";
import { RootModal } from "./components/modals/RootModal";
import { StatusBadge } from "./components/foundation/StatusBadge";
import { queryClient } from "./queryClient";

import type React from "react";

import "../public/global.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/800.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/700.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/500.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/400.css";

import "@left-curve/foundation/fonts/ABCDiatypeRounded/mono/600.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/mono/500.css";

import "@left-curve/foundation/fonts/Exposure/italic/400.css";
import "@left-curve/foundation/fonts/Exposure/italic/700.css";

export const App: React.FC = () => {
  return (
    <QueryClientProvider client={queryClient}>
      <DangoStoreProvider config={config}>
        <AppProvider toast={toast} navigate={(to, options) => router.navigate({ to, ...options })}>
          <AppRouter />
          <RootModal />
          <Toaster />
        </AppProvider>
      </DangoStoreProvider>
    </QueryClientProvider>
  );
};
