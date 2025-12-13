import type { Client, Transport } from "@left-curve/sdk/types";
import type { DangoClient, Signer } from "../../types/index.js";

import {
  transferRemote,
  type TransferRemoteParameters,
  type TransferRemoteReturnType,
} from "./mutations/transferRemote.js";

import {
  getWithdrawlFee,
  type GetWithdrawlFeeParameters,
  type GetWithdrawlFeeReturnType,
} from "./queries/getWithdrawalFee.js";

export type GatewayQueryActions = {
  gateway: {
    getWithdrawalFee: (parameters: GetWithdrawlFeeParameters) => GetWithdrawlFeeReturnType;
  };
};

export function gatewayQueryActions<transport extends Transport = Transport>(
  client: Client<transport>,
): GatewayQueryActions {
  return {
    gateway: {
      getWithdrawalFee: (...args) => getWithdrawlFee(client, ...args),
    },
  };
}

export type GatewayMutationActions = {
  gateway: {
    transferRemote: (parameters: TransferRemoteParameters) => TransferRemoteReturnType;
  };
};

export function gatewayMutationActions<transport extends Transport = Transport>(
  client: DangoClient<transport, Signer>,
): GatewayMutationActions {
  return {
    gateway: {
      transferRemote: (...args) => transferRemote(client, ...args),
    },
  };
}
