import { getAppConfig } from "@left-curve/sdk";
import { getMembersTypedData } from "../../../utils/typedData.js";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import type { Address, Transport, TxParameters } from "@left-curve/sdk/types";
import type {
  AccountConfig,
  AppConfig,
  DangoClient,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

export type RegisterAccountParameters = {
  sender: Address;
  config: AccountConfig;
};

export type RegisterAccountReturnType = ExecuteReturnType;

export async function registerAccount<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: RegisterAccountParameters,
  txParameters: TxParameters = {},
): RegisterAccountReturnType {
  const { sender, config } = parameters;
  const msg = { registerAccount: { params: config } };

  const { addresses } = await getAppConfig<AppConfig>(client);

  const typedData: TypedDataParameter = {
    type: [{ name: "register_account", type: "RegisterAccount" }],
    extraTypes: {
      RegisterAccount: [{ name: "params", type: "AccountParams" }],
      ...("spot" in config
        ? {
            AccountParams: [{ name: "spot", type: "SpotParams" }],
            SpotParams: [{ name: "owner", type: "string" }],
          }
        : {}),
      ...("multi" in config
        ? {
            AccountParams: [{ name: "multi", type: "SafeParams" }],
            SafeParams: [
              { name: "threshold", type: "uint32" },
              { name: "votingPeriod", type: "uint256" },
              { name: "timelock", type: "uint256" },
              { name: "members", type: "Member" },
            ],
            Member: [...getMembersTypedData(config.multi.members)],
          }
        : {}),
      ...("margin" in config
        ? {
            AccountParams: [{ name: "margin", type: "MarginParams" }],
            MarginParams: [{ name: "owner", type: "string" }],
          }
        : {}),
    },
  };

  return await execute(client, {
    contract: addresses.accountFactory,
    sender,
    msg,
    typedData,
    ...txParameters,
  });
}
