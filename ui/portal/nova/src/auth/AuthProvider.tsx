import { type ReactNode, createContext, useCallback, useMemo, useState } from "react";
import { useAccount } from "@left-curve/store";
import { AuthModal } from "./AuthModal";

import type { UseAccountReturnType } from "@left-curve/store";

type AuthContextValue = {
  readonly isAuthOpen: boolean;
  readonly showAuth: () => void;
  readonly hideAuth: () => void;
  readonly account: UseAccountReturnType;
};

export const AuthContext = createContext<AuthContextValue | null>(null);

type AuthProviderProps = {
  readonly children: ReactNode;
};

export function AuthProvider({ children }: AuthProviderProps) {
  const [isAuthOpen, setIsAuthOpen] = useState(false);
  const account = useAccount();

  const showAuth = useCallback(() => setIsAuthOpen(true), []);
  const hideAuth = useCallback(() => setIsAuthOpen(false), []);

  const value = useMemo(
    () => ({ isAuthOpen, showAuth, hideAuth, account }),
    [isAuthOpen, showAuth, hideAuth, account],
  );

  return (
    <AuthContext value={value}>
      {children}
      <AuthModal isOpen={isAuthOpen} onClose={hideAuth} />
    </AuthContext>
  );
}
