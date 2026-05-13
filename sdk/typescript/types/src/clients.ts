import type { Client, ClientConfig } from "./client.js";
import type { Signer } from "./signer.js";

/**
 * Placeholder for the public actions extension applied by `publicActions()` in dango.
 * Kept as a loose record so the types package has no dependency on dango.
 */
export type PublicActions = Record<string, unknown>;

/**
 * Placeholder for the signer actions extension applied by `signerActions()` in dango.
 * Kept as a loose record so the types package has no dependency on dango.
 */
export type SignerActions = Record<string, unknown>;

export type PublicClientConfig = ClientConfig<undefined>;

export type PublicClient = Client<undefined, PublicActions>;

export type SignerClientConfig = ClientConfig<Signer>;

export type SignerClient = Client<Signer, PublicActions & SignerActions>;
