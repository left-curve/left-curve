import type { Chain } from "@left-curve/dango/types";

declare global {
  interface Window {
    dango: {
      chain: Chain;
      urls: {
        faucetUrl: string;
        questUrl: string;
        upUrl: string;
        pointsUrl: string;
      };
      banner?: string;
      enabledFeatures?: string[];
    };
  }
}
