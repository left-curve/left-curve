import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import { execute } from "#actions/app/mutations/execute.js";
import { truncateDec } from "@left-curve/utils";

import type { Address, Client, Signer, TypedDataParameter } from "@left-curve/types";
import type { SignAndBroadcastTxReturnType } from "#actions/app/mutations/signAndBroadcastTx.js";

export type SetFeeShareRatioParameters = {
  sender: Address;
  shareRatio: string;
};

export type SetFeeShareRatioReturnType = SignAndBroadcastTxReturnType;

export async function setFeeShareRatio(
  client: Client<Signer>,
  parameters: SetFeeShareRatioParameters,
): SetFeeShareRatioReturnType {
  const { sender, shareRatio } = parameters;

  const truncatedRatio = truncateDec(shareRatio);

  const { addresses } = await getAppConfig(client);

  const msg = {
    referral: {
      setFeeShareRatio: {
        shareRatio: truncatedRatio,
      },
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "referral", type: "Referral" }],
    extraTypes: {
      Referral: [{ name: "set_fee_share_ratio", type: "SetFeeShareRatio" }],
      SetFeeShareRatio: [{ name: "share_ratio", type: "string" }],
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
