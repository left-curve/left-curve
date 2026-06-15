import type { Chain } from "@left-curve/types";

declare global {
  interface Window {
    dango: {
      chain: Chain;
      urls: {
        faucetUrl: string;
        upUrl: string;
        pointsUrl: string;
      };
      banner?: string;
      enabledFeatures?: string[];
    };
  }
}
