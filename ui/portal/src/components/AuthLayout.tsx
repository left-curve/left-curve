import { twMerge } from "@dango/shared";
import { useAccount } from "@left-curve/react";
import { ConnectionStatus } from "@left-curve/types";
import { motion } from "framer-motion";
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
          <div className="w-full h-full flex items-center justify-center md:justify-start md:items-start">
            <a href="/">
              <img src="/images/dango.svg" alt="logo" className="h-6 md:h-[31px] object-contain" />
            </a>
          </div>
          <motion.div
            animate={{
              background: isSignup
                ? "linear-gradient(90deg, #D88E96 0%, #C4B7BA 100%)"
                : "linear-gradient(90deg, #C2D0C9 0%, #93A4C8 100%)",
            }}
            transition={{ duration: 0.5 }}
            className="header w-full absolute h-[75px] md:h-[112px] top-0 left-0 z-[-1]"
          />
        </div>
        <div className="flex flex-1 w-full items-center justify-center p-4">
          <Outlet />
        </div>
      </div>
    </main>
  );
};
