"use client";

import { twMerge } from "@dango/shared";
import { usePathname } from "next/navigation";

export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const pathname = usePathname();
  const isSignup = pathname === "/auth/signup";

  return (
    <main className="flex flex-col min-h-screen w-full h-full bg-surface-off-white-200 relative overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center">
      <div className="min-h-full w-full flex-1 flex flex-col justify-center z-10 relative">
        <div
          className={twMerge(
            "header w-full sticky h-[80px] md:h-[130px] top-0 left-0 pt-4 pl-4 md:pt-7 md:pl-12 z-50",
            isSignup ? "header-signup" : "header-login",
          )}
        >
          <a href="/">
            <img src="/images/logo.webp" alt="logo" className="h-6 md:h-[31px] object-contain" />
          </a>
        </div>
        <div className="flex flex-1 w-full items-center justify-center p-4">{children}</div>
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
}
