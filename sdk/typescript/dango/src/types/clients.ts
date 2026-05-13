import type { Client, ClientConfig } from "./client.js";

import type { PublicActions } from "../actions/publicActions.js";
import type { SignerActions } from "../actions/signerActions.js";
import type { Signer } from "./signer.js";

export type PublicClientConfig = ClientConfig<undefined>;

export type PublicClient = Client<undefined, PublicActions>;

export type SignerClientConfig = ClientConfig<Signer>;

export type SignerClient = Client<Signer, PublicActions & SignerActions>;
