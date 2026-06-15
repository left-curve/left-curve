import { afterEach, describe, expect, it, vi } from "vitest";

import { createMipdStore, requestProviders } from "../../../store/src/mipd";

import type { EIP6963ProviderDetail } from "../../../store/src/types/eip6963";

function createProviderDetail({
  name,
  rdns,
  uuid,
}: {
  name: string;
  rdns: string;
  uuid: string;
}): EIP6963ProviderDetail {
  return {
    info: {
      icon: "data:image/svg+xml,<svg></svg>",
      name,
      rdns,
      uuid,
    },
    provider: {
      on: vi.fn(),
      removeListener: vi.fn(),
      request: vi.fn(),
    },
  };
}

function announceProvider(detail: EIP6963ProviderDetail) {
  window.dispatchEvent(new CustomEvent("eip6963:announceProvider", { detail }));
}

describe("EIP-6963 provider discovery store", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("requests announced providers and removes the announcement listener on unsubscribe", () => {
    const requestListener = vi.fn();
    const announcementListener = vi.fn();
    const provider = createProviderDetail({
      name: "Wallet One",
      rdns: "com.wallet.one",
      uuid: "wallet-one",
    });

    window.addEventListener("eip6963:requestProvider", requestListener);

    const unsubscribe = requestProviders(announcementListener);
    announceProvider(provider);

    expect(requestListener).toHaveBeenCalledOnce();
    expect(announcementListener).toHaveBeenCalledWith(provider);

    unsubscribe?.();
    announceProvider(
      createProviderDetail({
        name: "Wallet Two",
        rdns: "com.wallet.two",
        uuid: "wallet-two",
      }),
    );

    expect(announcementListener).toHaveBeenCalledOnce();

    window.removeEventListener("eip6963:requestProvider", requestListener);
  });

  it("deduplicates providers, debounces listeners, resets discovery, and tears down cleanly", () => {
    vi.useFakeTimers();

    const requestListener = vi.fn();
    window.addEventListener("eip6963:requestProvider", requestListener);

    const store = createMipdStore();
    const listener = vi.fn();
    const walletOne = createProviderDetail({
      name: "Wallet One",
      rdns: "com.wallet.one",
      uuid: "wallet-one",
    });
    const walletTwo = createProviderDetail({
      name: "Wallet Two",
      rdns: "com.wallet.two",
      uuid: "wallet-two",
    });

    const unsubscribe = store.subscribe(listener, { emitImmediately: true });

    announceProvider(walletOne);
    announceProvider(walletOne);
    announceProvider(walletTwo);

    expect(requestListener).toHaveBeenCalledOnce();
    expect(store.getProviders()).toEqual([walletOne, walletTwo]);
    expect(store.findProvider({ rdns: "com.wallet.two" })).toBe(walletTwo);
    expect(listener).toHaveBeenCalledOnce();

    vi.advanceTimersByTime(299);
    expect(listener).toHaveBeenCalledOnce();

    vi.advanceTimersByTime(1);
    expect(listener).toHaveBeenLastCalledWith([walletOne, walletTwo]);
    expect(listener).toHaveBeenCalledTimes(2);

    store.reset();

    expect(listener).toHaveBeenLastCalledWith([]);
    expect(store.getProviders()).toEqual([]);
    expect(requestListener).toHaveBeenCalledTimes(2);

    unsubscribe();
    announceProvider(walletOne);
    vi.advanceTimersByTime(300);

    expect(listener).toHaveBeenCalledTimes(3);

    store.destroy();
    expect(store._listeners().size).toBe(0);

    announceProvider(walletTwo);
    vi.advanceTimersByTime(300);

    expect(store.getProviders()).toEqual([]);

    window.removeEventListener("eip6963:requestProvider", requestListener);
  });
});
