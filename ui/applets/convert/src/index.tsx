import ReactDOM from "react-dom/client";
import { useState } from "react";
import { Convert } from "./components/Convert";
import { DangoRemoteProvider } from "@left-curve/store";
import { useRemoteApp, AppRemoteProvider } from "@left-curve/applets-kit";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";

import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/800.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/700.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/500.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/normal/400.css";

import "@left-curve/foundation/fonts/ABCDiatypeRounded/mono/600.css";
import "@left-curve/foundation/fonts/ABCDiatypeRounded/mono/500.css";

import "@left-curve/foundation/fonts/Exposure/italic/400.css";
import "@left-curve/foundation/fonts/Exposure/italic/700.css";
import "./global.css";

import type React from "react";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 0,
    },
  },
});

export const ConvertApplet: React.FC = () => {
  const appState = useRemoteApp();
  const [{ from, to }, onChangePair] = useState({ from: "USDC", to: "BTC" });

  return (
    <div className="w-full md:max-w-[25rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit text-primary-900">
      <Convert pair={{ from, to }} onChangePair={onChangePair} appState={appState}>
        <Convert.Header />
        <Convert.Form />
        <Convert.Details />
        <Convert.Trigger />
      </Convert>
    </div>
  );
};

const container = document.getElementById("root");
if (!container) throw new Error("No root element found");

const root = ReactDOM.createRoot(container);
root.render(
  <QueryClientProvider client={queryClient}>
    <DangoRemoteProvider>
      <AppRemoteProvider>
        <ConvertApplet />
      </AppRemoteProvider>
    </DangoRemoteProvider>
  </QueryClientProvider>,
);
