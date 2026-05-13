import type {
  Address,
  Client,
  GetTxMessage,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

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
