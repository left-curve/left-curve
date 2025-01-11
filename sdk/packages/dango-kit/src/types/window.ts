import type { EIP1193Provider } from "./eip1193.js";
import type { EIP6963AnnounceProviderEvent, EIP6963RequestProviderEvent } from "./eip6963.js";

declare global {
  interface Window {
    ethereum?: EIP1193Provider;
    keplr?: {
      ethereum: EIP1193Provider;
    };
  }
  interface WindowEventMap {
    "eip6963:announceProvider": EIP6963AnnounceProviderEvent;
    "eip6963:requestProvider": EIP6963RequestProviderEvent;
  }
}
