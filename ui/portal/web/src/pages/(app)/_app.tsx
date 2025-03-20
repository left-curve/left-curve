import { twMerge } from "@left-curve/applets-kit";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { Header } from "~/components/foundation/Header";

export const Route = createFileRoute("/(app)/_app")({
  component: function Layout() {
    const [isScrolled, setIsScrolled] = useState(false);
    const mainRef = useRef<HTMLElement | null>(null);

    useEffect(() => {
      const main = mainRef.current;
      if (!main) return;

      const handleScroll = () => {
        setIsScrolled(main.scrollTop > 70);
      };

      main.addEventListener("scroll", handleScroll);
      return () => main.removeEventListener("scroll", handleScroll);
    }, []);

    return (
      <main
        ref={mainRef}
        className="flex flex-col h-screen w-screen relative items-center justify-start overflow-y-auto overflow-x-hidden scrollbar-none bg-white-100"
      >
        <img
          src="/images/union.png"
          alt="bg-image"
          className={twMerge(
            "drag-none select-none h-[15vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0",
            isScrolled ? "z-40 lg:z-0" : "z-20 lg:z-0",
          )}
        />

        <Header isScrolled={isScrolled} />
        <div className="flex items-center justify-start w-full z-30 h-full relative flex-col">
          <Outlet />
        </div>
      </main>
    );
  },
});
