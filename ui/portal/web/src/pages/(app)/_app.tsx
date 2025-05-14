import { twMerge } from "@left-curve/applets-kit";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";
import { QuestBanner } from "~/components/foundation/QuestBanner";
import { WelcomeModal } from "~/components/modals/WelcomeModal";

export const Route = createFileRoute("/(app)/_app")({
  component: LayoutApp,
  errorComponent: () => {
    return (
      <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-y-auto overflow-x-hidden scrollbar-none bg-white-100">
        <img
          src="/images/union.png"
          alt="bg-image"
          className={twMerge(
            "drag-none select-none h-[15vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0",
          )}
        />
        <Header isScrolled={false} />
        <NotFound />
      </main>
    );
  },
});

function LayoutApp() {
  const [isScrolled, setIsScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      const scrollTop =
        window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0;

      setIsScrolled(scrollTop > 70);
    };

    window.addEventListener("scroll", handleScroll);
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <main className="flex flex-col w-full min-h-[100svh] relative pb-[3rem] lg:pb-0 max-w-screen">
      <img
        src="/images/union.png"
        alt="bg-image"
        className={twMerge(
          "pointer-events-none drag-none select-none h-[20vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0 object-cover object-bottom",
        )}
      />
      <QuestBanner />
      <WelcomeModal />
      <Header isScrolled={isScrolled} />
      <div className="flex flex-1 items-center justify-start w-full h-full relative flex-col z-30">
        <Outlet />
      </div>
    </main>
  );
}
