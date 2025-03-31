import type { FormatNumberOptions } from "@left-curve/dango/utils";
import { useAccount, useStorage } from "@left-curve/store";
import * as Sentry from "@sentry/react";
import {
  type Dispatch,
  type PropsWithChildren,
  type SetStateAction,
  createContext,
  useCallback,
  useEffect,
  useState,
} from "react";

type AppState = {
  isSidebarVisible: boolean;
  setSidebarVisibility: (visibility: boolean) => void;
  isNotificationMenuVisible: boolean;
  setNotificationMenuVisibility: (visibility: boolean) => void;
  isSearchBarVisible: boolean;
  setSearchBarVisibility: (visibility: boolean) => void;
  showModal: (modalName: string, modalProps?: any) => void;
  hideModal: () => void;
  formatNumberOptions: FormatNumberOptions;
  setFormatNumberOptions: Dispatch<SetStateAction<FormatNumberOptions>>;
  isModalVisible: boolean;
  activeModal?: string;
  modalProps: any;
};

export const AppContext = createContext<AppState | null>(null);

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { username } = useAccount();
  const [isSidebarVisible, setSidebarVisibility] = useState(false);
  const [isNotificationMenuVisible, setNotificationMenuVisibility] = useState(false);
  const [isSearchBarVisible, setSearchBarVisibility] = useState(false);
  const [activeModal, setSelectedModal] = useState<string>();
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [modalProps, setModalProps] = useState<Record<string, any>>({});
  const [formatNumberOptions, setFormatNumberOptions] = useStorage<FormatNumberOptions>(
    "formatNumber",
    {
      initialValue: {
        language: "en-US",
        maxFractionDigits: 2,
        minFractionDigits: 2,
        notation: "standard",
      },
    },
  );

  const showModal = useCallback((modalName: string, modalProps?: any) => {
    setModalProps(modalProps || {});
    setSelectedModal(modalName);
    setIsModalVisible(true);
  }, []);

  useEffect(() => {
    if (!username) Sentry.setUser(null);
    else Sentry.setUser({ username });
  }, [username]);

  const hideModal = useCallback(() => setIsModalVisible(false), [setIsModalVisible]);

  return (
    <AppContext.Provider
      value={{
        formatNumberOptions,
        setFormatNumberOptions,
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
  );
};
