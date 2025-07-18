import { twMerge, useTheme } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";
import { QuestBanner } from "~/components/foundation/QuestBanner";
import { WelcomeModal } from "~/components/modals/WelcomeModal";

export const Route = createFileRoute("/(app)/_app")({
  component: LayoutApp,
  errorComponent: ({ error }) => {
    useEffect(() => {
      captureException(error);
    }, []);

    const { theme } = useTheme();

    return (
      <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-x-hidden bg-surface-primary-rice">
        <img
          src={theme === "dark" ? "/images/union-dark.png" : "/images/union.png"}
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

  const { theme } = useTheme();

  return (
    <main className="flex flex-col w-full min-h-[100svh] relative pb-[3rem] lg:pb-0 max-w-screen bg-surface-primary-rice text-secondary-700">
      <img
        src={theme === "dark" ? "/images/union-dark.png" : "/images/union.png"}
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
