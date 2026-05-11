import { useRemoteApp as useAppRemoteProvider } from "../providers/AppRemoteProvider";
import { useApp as useAppProvider } from "../providers/AppProvider";

export function useApp(): ReturnType<typeof useAppProvider> {
  try {
    return useAppProvider();
  } catch {
    return useAppRemoteProvider();
  }
}
