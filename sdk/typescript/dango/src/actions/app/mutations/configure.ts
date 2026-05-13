import type { Address, GetTxMessage } from "../../../types/index.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type { Client } from "../../../types/client.js";
import type { Signer } from "../../../types/signer.js";
import type { TxMessageType, TypedDataParameter } from "../../../types/typedData.js";

type Message = GetTxMessage<"configure">;

export type ConfigureParameters = {
  sender: Address;
} & Message["configure"];

export type ConfigureReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function configure(
  client: Client<Signer>,
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
