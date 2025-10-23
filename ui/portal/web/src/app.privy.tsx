import { usePrivy, useWallets } from "@privy-io/react-auth";
import { type PropsWithChildren, useEffect } from "react";

import type React from "react";

export const AppPrivy: React.FC<PropsWithChildren> = ({ children }) => {
  const { wallets } = useWallets();
  const { logout, authenticated } = usePrivy();

  useEffect(() => {
    if (!authenticated) return;

    const wallet = wallets.find((w) => w.connectorType === "embedded");
    if (!wallet) return;

    (window as any).privy = Object.assign(wallet, { disconnect: logout });
  }, [wallets]);

  return children;
};
