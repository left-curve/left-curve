import { use } from "react";
import { AuthContext } from "./AuthProvider";

import type { UseAccountReturnType } from "@left-curve/store";

type UseAuthReturn = {
  readonly isAuthOpen: boolean;
  readonly showAuth: () => void;
  readonly hideAuth: () => void;
  readonly account: UseAccountReturnType;
};

export function useAuth(): UseAuthReturn {
  const context = use(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}
