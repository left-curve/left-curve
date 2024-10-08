import type { PropsWithChildren } from "react";
import { Header } from "./Header";

export const Layout: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-white relative overflow-y-auto overflow-x-hidden scrollbar-none items-center justify-center pt-[166px] md:pt-[110px]">
      <img
        src="/images/background.png"
        alt="bg-image"
        className="object-cover h-[80vh] absolute top-[15%] left-1/2 transform -translate-x-1/2 z-0 blur-2xl "
      />
      <Header />
      <main className="flex flex-1 w-full">{children}</main>
    </div>
  );
};
