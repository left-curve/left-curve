import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";

import type { Address, Client, Signer, TypedDataParameter } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";

export type SetReferralParameters = {
  sender: Address;
  referrer: number;
  referee: number;
};

export type SetReferralReturnType = SignAndBroadcastTxReturnType;

export async function setReferral(
  client: Client<Signer>,
  parameters: SetReferralParameters,
): SetReferralReturnType {
  const { sender, referrer, referee } = parameters;

  const { addresses } = await getAppConfig(client);

  const msg = {
    referral: {
      setReferral: {
        referrer,
        referee,
      },
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "referral", type: "Referral" }],
    extraTypes: {
      Referral: [{ name: "set_referral", type: "SetReferral" }],
      SetReferral: [
        { name: "referrer", type: "uint32" },
        { name: "referee", type: "uint32" },
      ],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      msg,
      typedData,
      contract: addresses.perps,
    },
  });
}
