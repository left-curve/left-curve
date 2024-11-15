import type { Chain, Client, Signer, Transport, TxParameters } from "@leftcurve/types";

import {
  type GetAllTokenAdminsParameters,
  type GetAllTokenAdminsReturnType,
  getAllTokenAdmins,
} from "./queries/getAllTokenAdmins.js";

import {
  type GetTokenAdminParameters,
  type GetTokenAdminReturnType,
  getTokenAdmin,
} from "./queries/getTokenAdmin.js";

import {
  type GetTokenFactoryConfigParameters,
  type GetTokenFactoryConfigReturnType,
  getTokenFactoryConfig,
} from "./queries/getTokenFactoryConfig.js";

import {
  type BurnTokenParameters,
  type BurnTokenReturnType,
  burnToken,
} from "./mutations/burnToken.js";

import {
  type CreateTokenParameters,
  type CreateTokenReturnType,
  createToken,
} from "./mutations/createToken.js";

import {
  type MintTokenParameters,
  type MintTokenReturnType,
  mintToken,
} from "./mutations/mintToken.js";

export type TokenFactoryActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = Signer,
> = {
  // queries
  getTokenFactoryConfig: (args: GetTokenFactoryConfigParameters) => GetTokenFactoryConfigReturnType;
  getTokenAdmin: (args: GetTokenAdminParameters) => GetTokenAdminReturnType;
  getAllTokenAdmins: (args: GetAllTokenAdminsParameters) => GetAllTokenAdminsReturnType;

  // mutations
  createToken: (args: CreateTokenParameters, txParameters: TxParameters) => CreateTokenReturnType;
  mintToken: (args: MintTokenParameters, txParameters: TxParameters) => MintTokenReturnType;
  burnToken: (args: BurnTokenParameters, txParameters: TxParameters) => BurnTokenReturnType;
};

export function ammActions<
  transport extends Transport = Transport,
  chain extends Chain | undefined = Chain,
  signer extends Signer = Signer,
>(client: Client<transport, chain, signer>): TokenFactoryActions<transport, chain, signer> {
  return {
    // queries
    getTokenFactoryConfig: (args: GetAllTokenAdminsParameters) =>
      getTokenFactoryConfig<chain, signer>(client, args),
    getTokenAdmin: (args: GetTokenAdminParameters) => getTokenAdmin<chain, signer>(client, args),
    getAllTokenAdmins: (args: GetAllTokenAdminsParameters) =>
      getAllTokenAdmins<chain, signer>(client, args),

    // mutations
    createToken: (args: CreateTokenParameters, txParameters: TxParameters) =>
      createToken<chain, signer>(client, args, txParameters),
    mintToken: (args: MintTokenParameters, txParameters: TxParameters) =>
      mintToken<chain, signer>(client, args, txParameters),
    burnToken: (args: BurnTokenParameters, txParameters: TxParameters) =>
      burnToken<chain, signer>(client, args, txParameters),
  };
}
