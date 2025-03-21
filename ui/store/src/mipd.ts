// This file type is partially forked from viem types in the following repository: https://github.com/wevm/mipd/blob/main/src/store.ts

import { debounce } from "@left-curve/dango/utils";

import type { EIP6963AnnounceProviderEvent, EIP6963ProviderDetail } from "./types/eip6963.js";
import type { MipdStore, MipdStoreListener } from "./types/mipd.js";
import "./types/window.js";

export function requestProviders(
  listener: (providerDetail: EIP6963ProviderDetail) => void,
): (() => void) | undefined {
  if (typeof window === "undefined") return;
  const handler = (event: EIP6963AnnounceProviderEvent) => listener(event.detail);

  window.addEventListener("eip6963:announceProvider", handler);

  window.dispatchEvent(new CustomEvent("eip6963:requestProvider"));

  return () => window.removeEventListener("eip6963:announceProvider", handler);
}

export function createMipdStore(): MipdStore {
  const listeners: Set<MipdStoreListener> = new Set();
  let providerDetails: readonly EIP6963ProviderDetail[] = [];

  const communicate = debounce((providerDetails) => {
    listeners.forEach((listener) => listener(providerDetails));
  }, 300);

  const request = () =>
    requestProviders((providerDetail) => {
      if (providerDetails.some(({ info }) => info.uuid === providerDetail.info.uuid)) return;

      providerDetails = [...providerDetails, providerDetail];
      communicate(providerDetails);
    });
  let unwatch = request();

  return {
    _listeners() {
      return listeners;
    },
    clear() {
      listeners.forEach((listener) => listener([]));
      providerDetails = [];
    },
    destroy() {
      this.clear();
      listeners.clear();
      unwatch?.();
    },
    findProvider({ rdns }) {
      return providerDetails.find((providerDetail) => providerDetail.info.rdns === rdns);
    },
    getProviders() {
      return providerDetails;
    },
    reset() {
      this.clear();
      unwatch?.();
      unwatch = request();
    },
    subscribe(listener, { emitImmediately } = {}) {
      listeners.add(listener);
      if (emitImmediately) listener(providerDetails);
      return () => listeners.delete(listener);
    },
  };
}
