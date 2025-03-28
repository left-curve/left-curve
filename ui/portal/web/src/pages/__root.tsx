import { createRootRouteWithContext } from "@tanstack/react-router";
import type { RouterContext } from "~/app.router";

export const Route = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context }) => {
    const { config } = context;
    if (!config?.state.isMipdLoaded) {
      await new Promise((resolve) => {
        config?.subscribe(
          (x) => x.isMipdLoaded,
          (isMipdLoaded) => isMipdLoaded && resolve(null),
        );
      });
    }
  },
  errorComponent: () => <div>Something went wrong</div>,
});
