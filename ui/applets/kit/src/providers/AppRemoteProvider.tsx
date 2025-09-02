import { type AppState, createContext } from "@left-curve/foundation";

import type { PropsWithChildren } from "react";
import type React from "react";

const [RemoteContextProvider, useRemoteApp] = createContext<AppState>();

export { useRemoteApp };

export const AppRemoteProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return <RemoteContextProvider value={{} as AppState}>{children}</RemoteContextProvider>;
};
