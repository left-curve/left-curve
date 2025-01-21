import type { Client, ClientConfig, RequiredBy, Transport } from "@left-curve/types";

import type { PublicActions } from "../actions/publicActions.js";
import type { SignerActions } from "../actions/signerActions.js";
import type { Chain } from "./chain.js";
import type { Signer } from "./signer.js";

export type PublicClientConfig<transport extends Transport = Transport> = ClientConfig<
  transport,
  Chain,
  undefined
>;

export type PublicClient<transport extends Transport = Transport> = Client<
  transport,
  Chain,
  undefined,
  PublicActions<transport>
>;

export type SignerClientConfig<transport extends Transport = Transport> = RequiredBy<
  ClientConfig<transport, Chain, Signer>,
  "signer"
> & { username: string };

export type SignerClient<transport extends Transport = Transport> = Client<
  transport,
  Chain,
  Signer,
  PublicActions<transport> & SignerActions & { username: string }
>;
