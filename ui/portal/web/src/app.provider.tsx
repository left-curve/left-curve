import { DangoStoreProvider } from "@left-curve/store-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type PropsWithChildren, createContext, useCallback, useState } from "react";
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
  isNotificationMenuVisible: boolean;
  setNotificationMenuVisibility: (visibility: boolean) => void;
  isSearchBarVisible: boolean;
  setSearchBarVisibility: (visibility: boolean) => void;
  showModal: (modalName: string, modalProps?: any) => void;
  hideModal: () => void;
  isModalVisible: boolean;
  activeModal?: string;
  modalProps: any;
};

export const AppContext = createContext<AppState | null>(null);

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isNotificationMenuVisible, setNotificationMenuVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [activeModal, setSelectedModal] = useState<string>();
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [modalProps, setModalProps] = useState<Record<string, any>>({});

  const showModal = useCallback((modalName: string, modalProps?: any) => {
    setModalProps(modalProps || {});
    setSelectedModal(modalName);
    setIsModalVisible(true);
  }, []);

  const hideModal = useCallback(() => setIsModalVisible(false), [setIsModalVisible]);

  return (
    <DangoStoreProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <AppContext.Provider
          value={{
            isSidebarVisible,
            setSidebarVisibility,
            isNotificationMenuVisible,
            setNotificationMenuVisibility,
            isSearchBarVisible,
            setSearchBarVisibility,
            showModal,
            hideModal,
            isModalVisible,
            activeModal,
            modalProps,
          }}
        >
          {children}
        </AppContext.Provider>
      </QueryClientProvider>
    </DangoStoreProvider>
  );
};
