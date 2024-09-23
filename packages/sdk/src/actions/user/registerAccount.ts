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
import { getMembersTypedData } from "@leftcurve/utils";
import { type ExecuteReturnType, execute, getAppConfig } from "~/actions";

export type RegisterAccountParameters = {
  sender: Address;
  config: AccountConfig;
};

export type RegisterAccountReturnType = ExecuteReturnType;

export async function registerAccount<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
  parameters: RegisterAccountParameters,
  txParameters: Pick<TxParameters, "gasLimit">,
): RegisterAccountReturnType {
  const { sender, config } = parameters;
  const msg = { registerAccount: { params: config } };

  const factoryAddr = await getAppConfig<Address>(client, { key: "account_factory" });

  const typedData: TypedDataParameter = {
    type: [{ name: "registerAccount", type: "RegisterAccount" }],
    extraTypes: {
      RegisterAccount: [{ name: "params", type: "AccountParams" }],
      AccountParams: [
        ...(config.safe
          ? [
              { name: "threshold", type: "uint32" },
              { name: "votingPeriod", type: "uint256" },
              { name: "timelock", type: "uint256" },
              { name: "members", type: "Member" },
            ]
          : [{ name: "owner", type: "string" }]),
      ],
      Member: [...(config?.safe?.members ? getMembersTypedData(config.safe.members) : [])],
    },
  };

  return await execute(client, { contract: factoryAddr, sender, msg, typedData, ...txParameters });
}
