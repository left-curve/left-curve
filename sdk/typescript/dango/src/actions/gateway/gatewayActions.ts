import type { Client, Signer } from "@left-curve/types";

import {
  transferRemote,
  type TransferRemoteParameters,
  type TransferRemoteReturnType,
} from "./mutations/transferRemote.js";

import {
  getWithdrawalFee,
  type GetWithdrawalFeeParameters,
  type GetWithdrawalFeeReturnType,
} from "./queries/getWithdrawalFee.js";

export type GatewayQueryActions = {
  gateway: {
    getWithdrawalFee: (parameters: GetWithdrawalFeeParameters) => GetWithdrawalFeeReturnType;
  };
};

export function gatewayQueryActions(client: Client): GatewayQueryActions {
  return {
    gateway: {
      getWithdrawalFee: (...args) => getWithdrawalFee(client, ...args),
    },
  };
}

export type GatewayMutationActions = {
  gateway: {
    transferRemote: (parameters: TransferRemoteParameters) => TransferRemoteReturnType;
  };
};

export function gatewayMutationActions(client: Client<Signer>): GatewayMutationActions {
  return {
    gateway: {
      transferRemote: (...args) => transferRemote(client, ...args),
    },
  };
}
