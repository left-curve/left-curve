import { getAppConfig } from "@left-curve/sdk";
import { getAction } from "@left-curve/sdk/actions";
import { execute } from "../../app/mutations/execute.js";

import type { Address, Transport } from "@left-curve/sdk/types";
import type { SignAndBroadcastTxReturnType } from "../../app/mutations/signAndBroadcastTx.js";
import type { AppConfig, DangoClient, Signer, TypedDataParameter } from "../../../types/index.js";

export type SetFeeShareRatioParameters = {
  sender: Address;
  shareRatio: string;
};

export type SetFeeShareRatioReturnType = SignAndBroadcastTxReturnType;

export async function setFeeShareRatio<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SetFeeShareRatioParameters,
): SetFeeShareRatioReturnType {
  const { sender, shareRatio } = parameters;

  const getAppConfigAction = getAction(client, getAppConfig, "getAppConfig");
  const { addresses } = await getAppConfigAction<AppConfig>({});

  const msg = {
    referral: {
      setFeeShareRatio: {
        shareRatio,
      },
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "referral", type: "Referral" }],
    extraTypes: {
      Referral: [{ name: "setFeeShareRatio", type: "SetFeeShareRatio" }],
      SetFeeShareRatio: [{ name: "shareRatio", type: "string" }],
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
