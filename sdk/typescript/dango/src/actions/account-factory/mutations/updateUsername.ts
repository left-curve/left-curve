import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import type { Address, Client, Signer, Username } from "@left-curve/types";
import { type ExecuteReturnType, execute } from "#actions/app/mutations/execute.js";

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

  const { addresses } = await getAppConfig(client);

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
