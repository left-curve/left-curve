import { Spinner } from "@left-curve/applets-kit";
import { deserializeJson } from "@left-curve/dango/encoding";
import { usePrivy, useWallets } from "@privy-io/react-auth";
import { type PropsWithChildren, useEffect, useState } from "react";

import type React from "react";

export const AppPrivy: React.FC<PropsWithChildren> = ({ children }) => {
  const [isReady, setIsReady] = useState(false);
  const { wallets, ready } = useWallets();
  const { logout, authenticated } = usePrivy();

  const theme = (() => {
    const theme = localStorage.getItem("dango.app.theme");
    const preferedSchema = window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
    if (!theme) return preferedSchema;
    const { value: themeSchema } = deserializeJson<{ value: string }>(theme);
    if (themeSchema === "system") return preferedSchema;
    return themeSchema;
  })();

  useEffect(() => {
    const root = window?.document.documentElement;
    root.classList.add(theme);
  }, []);

  useEffect(() => {
    if (!authenticated) return;

    const wallet = wallets.find((w) => w.connectorType === "embedded");
    if (!wallet) return;

    (window as any).privy = Object.assign(wallet, { disconnect: logout });
  }, [wallets]);

  useEffect(() => {
    if (!ready) return;
    setTimeout(() => setIsReady(true), 200);
  }, [ready]);

  return isReady ? (
    children
  ) : (
    <div className="w-screen h-screen">
      <Spinner fullContainer color="pink" size="lg" />
    </div>
  );
};
