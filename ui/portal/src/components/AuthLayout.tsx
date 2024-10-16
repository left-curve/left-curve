import { useAccount } from "@leftcurve/react";
import { ConnectionStatus } from "@leftcurve/types";
import { Navigate, Outlet } from "react-router-dom";

export const AuthLayout: React.FC = () => {
  const { status } = useAccount();

  if (status === ConnectionStatus.Connected) {
    return <Navigate to="/" />;
  }

  return (
    <main className="flex flex-col min-h-screen w-full h-full bg-white relative overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
      <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10 relative p-4">
        <a
          className="header w-full fixed h-[160px] top-0 py-4 md:py-7 px-6 md:px-12 z-50 left-0"
          href="/"
        >
          <img src="/images/logo.webp" alt="logo" className="h-6 md:h-[31px] object-contain" />
        </a>
        <div className="flex flex-1 w-full items-center justify-center">
          <Outlet />
        </div>
        <footer className="flex flex-col gap-10 items-center justify-center ">
          <div className="flex gap-12 uppercase font-extrabold">
            <a
              href="https://x.com/leftCurveSoft"
              target="_blank"
              rel="noreferrer"
              className="uppercase"
            >
              twitter
            </a>
            <a
              href="https://discord.gg/4uB9UDzYhz"
              target="_blank"
              rel="noreferrer"
              className="uppercase"
            >
              discord
            </a>
          </div>
          <div className="flex items-center justify-between md:justify-center text-xs font-light md:gap-12 px-4 w-full">
            <a href="/" className="uppercase">
              terms of use
            </a>
            <a href="/" className="uppercase">
              cookie policy
            </a>
            <a href="/" className="uppercase">
              privacy policy
            </a>
          </div>
        </footer>
      </div>
    </main>
  );
};
