import { useRemoteApp as useAppRemoteProvider } from "../providers/AppRemoteProvider";
import { useApp as useAppProvider } from "../providers/AppProvider";

export function useApp(): ReturnType<typeof useAppProvider> {
  try {
    // biome-ignore lint/correctness/useHookAtTopLevel: intentional call for AppProvider
    return useAppProvider();
  } catch {
    // biome-ignore lint/correctness/useHookAtTopLevel: intentional fallback when AppProvider is absent
    return useAppRemoteProvider();
  }
}
