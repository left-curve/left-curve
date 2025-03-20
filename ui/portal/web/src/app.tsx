import type React from "react";
import ReactDOM from "react-dom/client";
import { Toaster } from "react-hot-toast";
import { Modal } from "./components/foundation/Modal";

import { DangoStoreProvider } from "@left-curve/store-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "../store.config";

import { AppProvider } from "./app.provider";
import { AppRouter } from "./app.router";

import "../public/global.css";
import "@left-curve/ui-config/fonts/ABCDiatypeRounded/index.css";
import "@left-curve/ui-config/fonts/Exposure/index.css";

const queryClient = new QueryClient({
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

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
