import { createLazyRoute } from "@tanstack/react-router";
import { SwapContainer } from "~/components/SwapContainer";

export const SwapRoute = createLazyRoute("/swap")({
  component: () => {
    return (
      <div className="p-4 flex-1 flex items-center justify-center">
        <SwapContainer />
      </div>
    );
  },
});
