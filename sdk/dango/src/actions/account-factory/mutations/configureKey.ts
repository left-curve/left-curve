import { getAppConfig } from "@left-curve/sdk";
import type { Address, Hex, Transport } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import type {
  AppConfig,
  DangoClient,
  Key,
  KeyHash,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

export type ConfigureKeyParameters = {
  sender: Address;
  action: "delete" | { insert: Key };
  keyHash: KeyHash;
};

export type ConfigureKeyReturnType = ExecuteReturnType;

export type MsgConfigureKey = {
  configureKey: {
    KeyHash: Hex;
    key: "delete" | { insert: Key };
  };
};

export async function configureKey<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: ConfigureKeyParameters,
): ConfigureKeyReturnType {
  const { keyHash, action, sender } = parameters;

  const { addresses } = await getAppConfig<AppConfig>(client);

  const configureKeyMsg = {
    configureKey: {
      keyHash,
      key: action,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "configure_key", type: "ConfigureKey" }],
    extraTypes: {
      ConfigureKey: [
        { name: "key_hash", type: "string" },
        typeof action === "string"
          ? { name: "key", type: "string" }
          : { name: "key", type: "Operation" },
      ],
      ...(typeof action === "string"
        ? {}
        : {
            Operation: [{ name: "insert", type: "InsertOperation" }],
            InsertOperation: [
              // biome-ignore lint/complexity/useLiteralKeys: <explanation>
              { name: Object.keys(action["insert"]).at(0) as string, type: "string" },
            ],
          }),
    },
  };

  return await execute(client, {
    contract: addresses.accountFactory,
    sender,
    msg: configureKeyMsg,
    typedData,
  });
}
