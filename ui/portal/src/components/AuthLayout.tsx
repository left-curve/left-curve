import { twMerge } from "@dango/shared";
import { useAccount } from "@leftcurve/react";
import { ConnectionStatus } from "@leftcurve/types";
import { Navigate, Outlet, useLocation } from "react-router-dom";

export const AuthLayout: React.FC = () => {
  const { status } = useAccount();
  const location = useLocation();

  const isSignup = location.pathname === "/auth/signup";

  if (status === ConnectionStatus.Connected) {
    return <Navigate to="/" />;
  }

  return (
    <main className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 relative overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
      <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10 relative">
        <div
          className={twMerge(
            "header w-full sticky  h-[160px] top-0 left-0 pt-4 pl-4 md:pt-7 md:pl-12 z-50",
            isSignup ? "header-signup" : "header-login",
          )}
        >
          <a href="/">
            <img src="/images/logo.webp" alt="logo" className="h-6 md:h-[31px] object-contain" />
          </a>
        </div>
        <div className="flex flex-1 w-full items-center justify-center p-4">
          <Outlet />
        </div>
        <img
          src={isSignup ? "/images/chars/green-octopus.svg" : "/images/chars/purple-bird.svg"}
          alt="character"
          className={twMerge(
            "hidden md:block  object-contain absolute bottom-[3.75rem] ",
            isSignup ? "max-w-[16rem] w-[16%] left-[10%]" : "max-w-[23rem] w-[23%] left-[3%]",
          )}
        />
      </div>
    </main>
  );
};
