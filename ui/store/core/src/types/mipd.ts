import type { EIP6963ProviderDetail } from "./eip6963.js";

export type MipdStoreListener = (providerDetails: readonly EIP6963ProviderDetail[]) => void;

export type MipdStore = {
  /**
   * Clears the store, including all provider details.
   */
  clear(): void;
  /**
   * Destroys the store, including all provider details and listeners.
   */
  destroy(): void;
  /**
   * Finds a provider detail by its RDNS (Reverse Domain Name Identifier).
   */
  findProvider(args: { rdns: string }): EIP6963ProviderDetail | undefined;
  /**
   * Returns all provider details that have been emitted.
   */
  getProviders(): readonly EIP6963ProviderDetail[];
  /**
   * Resets the store, and emits an event to request provider details.
   */
  reset(): void;
  /**
   * Subscribes to emitted provider details.
   */
  subscribe(
    listener: MipdStoreListener,
    args?: { emitImmediately?: boolean | undefined } | undefined,
  ): () => void;

  /**
   * @internal
   * Current state of listening listeners.
   */
  _listeners(): Set<MipdStoreListener>;
};
