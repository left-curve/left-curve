import { getAppConfig } from "../../../index.js";
import type { Address } from "../../../types/index.js";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import { getAction } from "../../index.js";
import type { AppConfig } from "../../../types/app.js";
import type { Client } from "../../../types/client.js";
import type { Signer } from "../../../types/signer.js";
import type { Username } from "../../../types/account.js";

export type UpdateUsernameParameters = {
  sender: Address;
  username: Username;
};

export type UpdateUsernameReturnType = ExecuteReturnType;

export async function updateUsername(
  client: Client<Signer>,
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
