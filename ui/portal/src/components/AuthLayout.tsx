import { twMerge } from "@dango/shared";
import { useAccount } from "@left-curve/react";
import { ConnectionStatus } from "@left-curve/types";
import { Navigate, Outlet, useLocation } from "react-router-dom";

export const AuthLayout: React.FC = () => {
  const { status } = useAccount();
  const location = useLocation();

  const isSignup = location.pathname === "/auth/signup";

  if (status === ConnectionStatus.Connected) {
    return <Navigate to="/" />;
  }

  return (
    <main className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
      <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10">
        <div className="relative h-[70px] md:h-[112px] px-12 py-7">
          <a
            href="/"
            className="w-full h-full flex items-center justify-center md:justify-start md:items-start"
          >
            <img src="/images/dango.svg" alt="logo" className="h-6 md:h-[31px] object-contain" />
          </a>
          <div
            className={twMerge(
              "header w-full absolute h-[75px] md:h-[112px] top-0 left-0 z-[-1]",
              isSignup ? "header-signup" : "header-login",
            )}
          />
        </div>
        <div className="flex flex-1 w-full items-center justify-center p-4">
          <Outlet />
        </div>
        <img
          src={isSignup ? "/images/chars/green-octopus.svg" : "/images/chars/purple-bird.svg"}
          alt="character"
          className={twMerge(
            "hidden lg:block  object-contain absolute bottom-[3.75rem] ",
            isSignup ? "max-w-[16rem] w-[16%] left-[10%]" : "max-w-[23rem] w-[23%] left-[3%]",
          )}
        />
      </div>
    </main>
  );
};
