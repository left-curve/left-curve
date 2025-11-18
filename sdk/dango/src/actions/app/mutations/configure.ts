import type { Address, GetTxMesssage, Transport } from "@left-curve/sdk/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

type Message = GetTxMesssage<"configure">;

export type ConfigureParameters = {
  sender: Address;
} & Message["configure"];

export type ConfigureReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function configure<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: ConfigureParameters,
): ConfigureReturnType {
  const { sender, newAppCfg, newCfg } = parameters;

  const message: Message = {
    configure: {
      newAppCfg,
      newCfg,
    },
  };

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "configure", type: "Configure" }],
    extraTypes: {
      Configure: [
        { name: "new_app_cfg", type: "AppConfig" },
        { name: "new_cfg", type: "Config" },
      ],
      AppConfig: [],
      Config: [],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [message], typedData });
}
