import type { Transport } from "@left-curve/sdk/types";
import type { DangoClient, Signer } from "../../types/index.js";

import {
  transferRemote,
  type TransferRemoteParameters,
  type TransferRemoteReturnType,
} from "./mutations/transferRemote.js";

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
