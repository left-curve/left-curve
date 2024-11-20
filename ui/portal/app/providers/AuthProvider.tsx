"use client";

import { useAccount, useConfig } from "@leftcurve/react";
import { ConnectionStatus } from "@leftcurve/types";
import { type PropsWithChildren, useEffect, useLayoutEffect, useState } from "react";

import { Spinner } from "@dango/shared";
import { redirect } from "next/navigation";

export const AuthProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { subscribe, state } = useConfig();
  const { status } = useAccount();

  const [isLoading, setIsLoading] = useState(!state.isMipdLoaded);

  subscribe(
    (x) => x.isMipdLoaded,
    (isMipdLoaded) => setIsLoading(!isMipdLoaded),
  );

  useLayoutEffect(() => {
    if (isLoading) return;
    if (status !== ConnectionStatus.Connected) redirect("/auth/login");
  }, [status]);

  if (isLoading) {
    return (
      <div className="h-screen w-full flex justify-center items-center">
        <Spinner size="lg" color="pink" />
      </div>
    );
  }

  return <>{children}</>;
};
