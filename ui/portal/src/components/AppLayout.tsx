import { useAccount, useConfig } from "@leftcurve/react";
import { Navigate, Outlet } from "react-router-dom";

import { Spinner } from "@dango/shared";
import { ConnectionStatus } from "@leftcurve/types";
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
    <div className="flex flex-col min-h-screen w-full h-full bg-white relative scrollbar-none items-center justify-center">
      {status === ConnectionStatus.Connected ? (
        <img
          src="/images/background.png"
          alt="bg-image"
          className="object-cover h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl "
        />
      ) : null}
      <Header />
      <main className="flex flex-1 w-full">
        {status === ConnectionStatus.Connected ? <Outlet /> : <Navigate to="/auth/login" />}
      </main>
    </div>
  );
};
