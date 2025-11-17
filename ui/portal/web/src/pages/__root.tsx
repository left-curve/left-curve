import { HeadContent, Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { useEffect } from "react";
import { useAccount, useActivities, useSessionKey } from "@left-curve/store";

import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";

import * as Sentry from "@sentry/react";
import { Modals, twMerge, useApp, useTheme } from "@left-curve/applets-kit";
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
    const { modal, settings, showModal } = useApp();

    // Track user errors
    const { username, connector, account, isConnected } = useAccount();
    useEffect(() => {
      if (!username) Sentry.setUser(null);
      else {
        Sentry.setUser({ username });
        Sentry.setContext("connector", {
          id: connector?.id,
          name: connector?.name,
          type: connector?.type,
        });
      }
    }, [username, connector]);

    // Initialize activities
    const { startActivities } = useActivities();
    useEffect(() => {
      const stopActivities = startActivities();
      return stopActivities;
    }, [account]);

    // Track session key expiration
    const { session } = useSessionKey();
    useEffect(() => {
      const intervalId = setInterval(() => {
        if (
          (!session || Date.now() > Number(session.sessionInfo.expireAt)) &&
          isConnected &&
          settings.useSessionKey &&
          connector &&
          connector.type !== "session"
        ) {
          if (modal.modal !== Modals.RenewSession) {
            showModal(Modals.RenewSession);
          }
        }
      }, 1000);
      return () => {
        clearInterval(intervalId);
      };
    }, [session, modal, settings.useSessionKey, connector, isConnected]);

    return (
      <>
        {createPortal(<HeadContent />, document.querySelector("head")!)}
        <Outlet />
      </>
    );
  },
  errorComponent: ({ error }) => {
    const { theme } = useTheme();

    useEffect(() => {
      Sentry.captureException(error);
    }, []);

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
