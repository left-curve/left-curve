import { DangoStoreProvider } from "@left-curve/store";
import { captureException } from "@sentry/react";
import { MutationCache, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { config } from "~/store";

import { AppRouter, router } from "./app.router";
import { AppProvider } from "@left-curve/foundation";
import { Toast } from "@left-curve/applets-kit";
import { createToaster } from "./app.toaster";
import { RootModal } from "./components/modals/RootModal";

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

const [Toaster, toast] = createToaster((props) => <Toast {...props} />);

const channel = new BroadcastChannel("dango.queries");

channel.onmessage = ({ data: event }) => {
  if (event.type === "invalidate") {
    for (const key of event.keys) {
      queryClient.invalidateQueries({ queryKey: key });
    }
  }
};

const queryClient = new QueryClient({
  mutationCache: new MutationCache({
    onSettled(_data, _error, _variables, _context, mutation) {
      if (!mutation.meta?.invalidateKeys) return;
      channel.postMessage({ type: "invalidate", keys: mutation.meta.invalidateKeys });
    },
    onError: (error: unknown) => {
      if (!error) return;
      if (typeof error === "object" && ("code" in error || !(error instanceof Error))) return;
      if (typeof error === "string" && error.includes("reject")) return;

      const errorMessage = error instanceof Error ? error.message : error;
      captureException(errorMessage);
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
    <QueryClientProvider client={queryClient}>
      <DangoStoreProvider config={config}>
        <AppProvider toast={toast} navigate={(to, options) => router.navigate({ to, ...options })}>
          <AppRouter />
          <RootModal />
          <Toaster position="bottom-center" containerStyle={{ zIndex: 99999999 }} />
        </AppProvider>
      </DangoStoreProvider>
    </QueryClientProvider>
  );
};
