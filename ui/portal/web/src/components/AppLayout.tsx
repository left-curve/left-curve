import type { PropsWithChildren } from "react";
import { Header } from "./Header";

export const AppLayout: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="flex flex-col min-h-screen w-full h-full bg-[#FFFCF6] relative scrollbar-none items-center justify-center">
      <img
        src="/images/union.png"
        alt="bg-image"
        className="drag-none select-none h-[20vh] w-full absolute top-0 z-0"
      />

      <Header />
      <main className="flex flex-1 w-full z-[2]">{children}</main>
    </div>
  );
};
