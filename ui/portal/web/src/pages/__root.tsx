import {
  HeadContent,
  Outlet,
  createRootRouteWithContext,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";

import { Spinner, twMerge, useTheme } from "@left-curve/applets-kit";
import { createPortal } from "react-dom";

import type { RouterContext } from "~/app.router";

export const Route = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context }) => {
    const { config } = context;
    if (!config.state.isMipdLoaded) {
      await new Promise((resolve) => {
        config?.subscribe(
          (x) => x.isMipdLoaded,
          (isMipdLoaded) => isMipdLoaded && resolve(null),
        );
      });
    }
  },
  component: () => {
    const navigate = useNavigate();
    const { location } = useRouterState();
    const [isReady, setIsReady] = useState(false);
    useEffect(() => {
      if (location.pathname === "/maintenance") navigate({ to: "/" });
      (async () => {
        try {
          // Check chain is up
          const response = await fetch(window.dango.urls.upUrl);
          if (!response.ok) throw new Error("request failed");
          const { is_running } = await response.json();
          if (!is_running) navigate({ to: "/maintenance" });
        } catch (_) {
          navigate({ to: "/maintenance" });
        } finally {
          setIsReady(true);
        }
      })();
    }, []);

    if (!isReady) return <Spinner />;

    return (
      <>
        {createPortal(<HeadContent />, document.querySelector("head")!)}
        <Outlet />
      </>
    );
  },
  errorComponent: () => {
    const { theme } = useTheme();
    return (
      <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-y-auto overflow-x-hidden bg-surface-primary-rice">
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
