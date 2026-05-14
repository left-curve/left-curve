import type { ClientConfig } from "./client.js";
import type { Signer } from "./signer.js";

export type PublicClientConfig = ClientConfig<undefined>;

export type SignerClientConfig = ClientConfig<Signer>;
