import { twMerge, useApp, useMediaQuery, useTheme } from "@left-curve/applets-kit";
import { captureException } from "@sentry/react";
import { Outlet, createFileRoute, useRouter } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";
import { QuestBannerRender } from "~/components/foundation/QuestBanner";
import { StatusBadge } from "~/components/foundation/StatusBadge";
import { TestnetBanner } from "~/components/foundation/TestnetBanner";

import { WelcomeModal } from "~/components/modals/WelcomeModal";

export const Route = createFileRoute("/(app)/_app")({
  component: LayoutApp,
  errorComponent: ({ error }) => {
    useEffect(() => {
      captureException(error);
    }, []);

    const { theme } = useTheme();
    const { isLg } = useMediaQuery();

    return (
      <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-x-hidden bg-surface-primary-rice text-ink-secondary-700">
        <img
          src={theme === "dark" ? "/images/union-dark.png" : "/images/union.png"}
          alt="bg-image"
          className={twMerge(
            "drag-none select-none h-[15vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0",
            { hidden: location.pathname === "/" && !isLg },
          )}
        />
        <Header isScrolled={false} />
        <NotFound />
        <StatusBadge />
      </main>
    );
  },
});

function LayoutApp() {
  const [isScrolled, setIsScrolled] = useState(false);
  const { isLg } = useMediaQuery();
  const router = useRouter();
  const { isSidebarVisible } = useApp();

  const isProSwap = useMemo(() => {
    return router.state.location.pathname.includes("trade");
  }, [router.state.location.pathname]);

  useEffect(() => {
    const handleScroll = () => {
      const headerThreshold = isProSwap ? 20 : 70;

      const scrollTop =
        window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0;

      setIsScrolled(scrollTop > headerThreshold);
    };

    window.addEventListener("scroll", handleScroll);
    return () => window.removeEventListener("scroll", handleScroll);
  }, [isProSwap]);

  const { theme } = useTheme();

  const isHomePage = location.pathname === "/";
  const lockedY = Number(document.body.dataset.scrollLockY || 0);

  const effectiveIsScrolled = isSidebarVisible ? lockedY > (isProSwap ? 20 : 70) : isScrolled;

  return (
    <main className="flex flex-col w-full min-h-[100svh] relative pb-[3rem] lg:pb-0 max-w-screen bg-surface-primary-rice text-ink-secondary-700">
      <img
        src={theme === "dark" ? "/images/union-dark.png" : "/images/union.png"}
        alt="bg-image"
        className={twMerge(
          "pointer-events-none drag-none select-none h-[20vh] lg:h-[20vh] w-full fixed lg:absolute bottom-0 lg:top-0 left-0 z-40 lg:z-0 rotate-180 lg:rotate-0 object-cover object-bottom",
          { hidden: isHomePage && !isLg },
        )}
      />
      <WelcomeModal />
      {!isLg ? <div id="quest-banner-mobile" /> : null}
      {!isLg ? <TestnetBanner /> : null}
      <QuestBannerRender />
      <Header isScrolled={effectiveIsScrolled} />
      <div className="flex flex-1 items-center justify-start w-full h-full relative flex-col z-30">
        <Outlet />
      </div>
      <StatusBadge />
    </main>
  );
}
