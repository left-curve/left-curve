import type { Client, ClientConfig, ClientExtend, Transport } from "@left-curve/sdk/types";

import type { PublicActions } from "../actions/publicActions.js";
import type { SignerActions } from "../actions/signerActions.js";
import type { Chain } from "./chain.js";
import type { Signer } from "./signer.js";

export type DangoClient<
  transport extends Transport = Transport,
  signer extends Signer | undefined = Signer | undefined,
  extended extends ClientExtend = ClientExtend,
> = Client<transport, Chain, signer, { username: string } & extended>;

export type PublicClientConfig<transport extends Transport = Transport> = ClientConfig<
  transport,
  Chain,
  undefined
>;

export type PublicClient<transport extends Transport = Transport> = DangoClient<
  transport,
  undefined,
  PublicActions
>;

export type SignerClientConfig<transport extends Transport = Transport> = ClientConfig<
  transport,
  Chain,
  Signer,
  { username: string }
>;

export type SignerClient<transport extends Transport = Transport> = DangoClient<
  transport,
  Signer,
  PublicActions & SignerActions
>;
