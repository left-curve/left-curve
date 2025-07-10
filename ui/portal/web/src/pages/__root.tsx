import { HeadContent, Outlet, createRootRouteWithContext } from "@tanstack/react-router";

import { Header } from "~/components/foundation/Header";
import { NotFound } from "~/components/foundation/NotFound";

import { twMerge } from "@left-curve/applets-kit";
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
  component: () => (
    <>
      {createPortal(<HeadContent />, document.querySelector("head")!)}
      <Outlet />
    </>
  ),
  errorComponent: () => (
    <main className="flex flex-col h-screen w-screen relative items-center justify-start overflow-y-auto overflow-x-hidden bg-bg-primary-rice">
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
  ),
});
