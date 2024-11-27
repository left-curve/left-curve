import { useAccount, useConfig } from "@left-curve/react";
import { Navigate, Outlet } from "react-router-dom";

import { Spinner } from "@dango/shared";
import { ConnectionStatus } from "@left-curve/types";
import { useState } from "react";
import { Header } from "./Header";

export const AppLayout: React.FC = () => {
  const { subscribe, state } = useConfig();
  const { status } = useAccount();

  const [isLoading, setIsLoading] = useState(!state.isMipdLoaded);

  subscribe(
    (x) => x.isMipdLoaded,
    (isMipdLoaded) => setIsLoading(!isMipdLoaded),
  );

  if (isLoading)
    return (
      <div className="h-screen w-full flex justify-center items-center">
        <Spinner size="lg" color="pink" />
      </div>
    );

  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 relative scrollbar-none items-center justify-center">
      {status === ConnectionStatus.Connected ? (
        <img
          src="/images/background.png"
          alt="bg-image"
          className="object-cover drag-none select-none h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl opacity-40"
        />
      ) : null}
      <Header />
      <main className="flex flex-1 w-full z-[2]">
        {status === ConnectionStatus.Connected ? <Outlet /> : <Navigate to="/auth/login" />}
      </main>
    </div>
  );
};
