import { DangoStoreProvider } from "@left-curve/store-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type PropsWithChildren, createContext, useState } from "react";
import { config } from "../store.config";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 0,
    },
  },
});

type AppState = {
  isSidebarVisible: boolean;
  setSidebarVisibility: (visibility: boolean) => void;
};

export const AppContext = createContext<AppState | null>(null);

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const [isSidebarVisible, setSidebarVisibility] = useState(false);

  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <AppContext.Provider value={{ isSidebarVisible, setSidebarVisibility }}>
          {children}
        </AppContext.Provider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
