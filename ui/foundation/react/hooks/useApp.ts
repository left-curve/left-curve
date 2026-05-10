import { useContext } from "react";

import { AppContext } from "../providers/AppProvider";
import { AppRemoteContext } from "../providers/AppRemoteProvider";

import type { AppState } from "../providers/AppProvider";

export function useApp(): AppState {
  const local = useContext(AppContext);
  const remote = useContext(AppRemoteContext);
  const ctx = local ?? remote;
  if (!ctx) {
    throw new Error("useApp must be used inside <AppProvider> or <AppRemoteProvider>.");
  }
  return ctx;
}
