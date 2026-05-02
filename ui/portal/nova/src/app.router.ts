import type { UseThemeReturnType } from "@left-curve/applets-kit";
import type { QueryClient } from "@tanstack/react-query";
import type {
  UseAccountReturnType,
  UseConfigReturnType,
  UsePublicClientReturnType,
} from "@left-curve/store";

export interface RouterContext {
  client: UsePublicClientReturnType;
  account: UseAccountReturnType;
  config: UseConfigReturnType;
  theme: UseThemeReturnType;
  queryClient: QueryClient;
}
