import { getAppConfig } from "../../../index.js";
import { getAction } from "../../index.js";
import { execute } from "../../app/mutations/execute.js";
import { truncateDec } from "../../../utils/index.js";

import type { Address } from "../../../types/index.js";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type { AppConfig, Client, Signer, TypedDataParameter } from "../../../types/index.js";

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

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

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
