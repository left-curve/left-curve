import { getAppConfig } from "@left-curve/sdk";
import type { Address, Transport } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import { getAction } from "@left-curve/sdk/actions";
import type { AppConfig } from "../../../types/app.js";
import type { DangoClient } from "../../../types/clients.js";
import type { Signer } from "../../../types/signer.js";
import type { Username } from "../../../types/account.js";

export type UpdateUsernameParameters = {
  sender: Address;
  username: Username;
};

export type UpdateUsernameReturnType = ExecuteReturnType;

export async function updateUsername<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: UpdateUsernameParameters,
): UpdateUsernameReturnType {
  const { sender, username } = parameters;

  const getter = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getter<AppConfig>({});

  const msg = {
    updateUsername: username,
  };

  return await execute(client, {
    execute: {
      contract: addresses.accountFactory,
      msg,
    },
    sender,
  });
}
