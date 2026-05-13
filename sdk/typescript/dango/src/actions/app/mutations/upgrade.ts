import type {
  Address,
  Client,
  GetTxMessage,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

type Message = GetTxMessage<"upgrade">;

export type UpgradeParameters = {
  sender: Address;
} & Message["upgrade"];

export type UpgradeReturnType = Promise<SignAndBroadcastTxReturnType>;

export async function upgrade(
  client: Client<Signer>,
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
