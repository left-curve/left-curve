import { useAccount } from "@leftcurve/react";
import { Navigate, Outlet } from "react-router-dom";

import { Header } from "./Header";

export const AppLayout: React.FC = () => {
  const { status } = useAccount();

  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-white relative scrollbar-none items-center justify-center">
      {status === "connected" ? (
        <img
          src="/images/background.png"
          alt="bg-image"
          className="object-cover h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl "
        />
      ) : null}
      <Header />
      <main className="flex flex-1 w-full">
        {status === "connected" ? <Outlet /> : <Navigate to="/auth/login" />}
      </main>
    </div>
  );
};
