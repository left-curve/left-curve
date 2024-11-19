import { getMembersTypedData } from "@leftcurve/utils";
import { type ExecuteReturnType, execute, getAppConfig } from "../index.js";

import type {
  AccountConfig,
  Address,
  Chain,
  Client,
  Signer,
  Transport,
  TxParameters,
  TypedDataParameter,
} from "@leftcurve/types";
import type { DangoAppConfigResponse } from "@leftcurve/types/dango";

export type RegisterAccountParameters = {
  sender: Address;
  config: AccountConfig;
};

export type RegisterAccountReturnType = ExecuteReturnType;

export async function registerAccount<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: RegisterAccountParameters,
  txParameters: TxParameters = {},
): RegisterAccountReturnType {
  const { sender, config } = parameters;
  const msg = { registerAccount: { params: config } };

  const { addresses } = await getAppConfig<DangoAppConfigResponse>(client);

  const typedData: TypedDataParameter = {
    type: [{ name: "registerAccount", type: "RegisterAccount" }],
    extraTypes: {
      RegisterAccount: [{ name: "params", type: "AccountParams" }],
      ...("spot" in config
        ? {
            AccountParams: [{ name: "spot", type: "SpotParams" }],
            SpotParams: [{ name: "owner", type: "string" }],
          }
        : {}),
      ...("safe" in config
        ? {
            AccountParams: [{ name: "safe", type: "SafeParams" }],
            SafeParams: [
              { name: "threshold", type: "uint32" },
              { name: "votingPeriod", type: "uint256" },
              { name: "timelock", type: "uint256" },
              { name: "members", type: "Member" },
            ],
            Member: [...getMembersTypedData(config.safe.members)],
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
