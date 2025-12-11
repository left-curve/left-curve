import { getAppConfig } from "@left-curve/sdk";
import type { Address, Transport } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import { getAction } from "@left-curve/sdk/actions";

import type {
  AppConfig,
  DangoClient,
  Remote,
  Addr32,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

export type TransferRemoteParameters = {
  remote: Remote;
  recipient: Addr32;
  sender: Address;
};

export type TransferRemoteReturnType = ExecuteReturnType;

export async function transferRemote<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: TransferRemoteParameters,
): TransferRemoteReturnType {
  const { remote, recipient, sender } = parameters;

  const getter = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getter<AppConfig>({});

  const msg = {
    transferRemote: {
      remote,
      recipient,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "transfer_remote", type: "TransferRemote" }],
    extraTypes: {
      TransferRemote: [
        { name: "recipient", type: "string" },
        typeof remote === "string"
          ? { name: "remote", type: "string" }
          : { name: "remote", type: "Remote" },
      ],
      ...(typeof remote === "string"
        ? {}
        : {
            Remote: [{ name: "warp", type: "Warp" }],
            Warp: [
              { name: "domain", type: "uint32" },
              { name: "contract", type: "string" },
            ],
          }),
    },
  };

  return await execute(client, {
    execute: {
      contract: addresses.gateway,
      msg,
      typedData,
    },
    sender,
  });
}
