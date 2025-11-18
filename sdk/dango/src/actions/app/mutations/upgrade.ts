import type { Address, GetTxMessage, Transport } from "@left-curve/sdk/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

type Message = GetTxMessage<"upgrade">;

export type UpgradeParameters = {
  sender: Address;
} & Message["upgrade"];

export type UpgradeReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function upgrade<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: UpgradeParameters,
): UpgradeReturnType {
  const { sender, height, cargoVersion, gitTag, url } = parameters;

  const message: Message = {
    upgrade: {
      height,
      cargoVersion,
      gitTag,
      url,
    },
  };

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "upgrade", type: "Upgrade" }],
    extraTypes: {
      upgrade: [
        { name: "height", type: "uint32" },
        { name: "cargo_version", type: "string" },
        ...(gitTag ? [{ name: "git_tag", type: "string" }] : []),
        ...(url ? [{ name: "url", type: "string" }] : []),
      ],
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [message], typedData });
}
