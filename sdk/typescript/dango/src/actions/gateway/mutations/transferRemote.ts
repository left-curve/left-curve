import { getAppConfig } from "../../../index.js";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";
import { getAction } from "../../index.js";

import type { Address, Coins } from "../../../types/index.js";
import type { AppConfig } from "../../../types/app.js";
import type { Client } from "../../../types/client.js";
import type { Remote, Addr32 } from "../../../types/hyperlane.js";
import type { Signer } from "../../../types/signer.js";
import type { TypedDataParameter } from "../../../types/typedData.js";

export type TransferRemoteParameters = {
  remote: Remote;
  recipient: Addr32;
  sender: Address;
  funds: Coins;
};

export type TransferRemoteReturnType = ExecuteReturnType;

export async function transferRemote(
  client: Client<Signer>,
  parameters: TransferRemoteParameters,
): TransferRemoteReturnType {
  const { remote, recipient, sender, funds } = parameters;

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
      funds,
    },
    sender,
  });
}
