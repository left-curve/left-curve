import { HeadContent, Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import { useEffect } from "react";
import {
  getAppConfigQueryOptions,
  useAccount,
  useActivities,
  useSessionKey,
} from "@left-curve/store";

import * as Sentry from "@sentry/react";
import { Modals, useApp } from "@left-curve/applets-kit";
import { createPortal } from "react-dom";
import { ErrorPage } from "~/components/foundation/ErrorPage";

import type { RouterContext } from "~/app.router";

export const Route = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context }) => {
    const { config, queryClient } = context;
    if (!config.state.isMipdLoaded) {
      await new Promise((resolve) => {
        config?.subscribe(
          (x) => x.isMipdLoaded,
          (isMipdLoaded) => isMipdLoaded && resolve(null),
        );
      });
    }
    await queryClient.ensureQueryData(getAppConfigQueryOptions(config, {})).catch(() => {});
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
          (!session || Date.now() > Number(session.sessionInfo.expireAt) * 1000) &&
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
  errorComponent: ({ error, reset }) => {
    return <ErrorPage error={error} reset={reset} />;
  },
});
