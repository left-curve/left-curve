import { getAppConfig } from "@left-curve/sdk";
import type { Address, Hex, Transport } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import { getAction } from "@left-curve/sdk/actions";
import type {
  AppConfig,
  DangoClient,
  Key,
  KeyHash,
  Signer,
  TypedDataParameter,
} from "../../../types/index.js";

export type UpdateKeyParameters = {
  sender: Address;
  action: "delete" | { insert: Key };
  keyHash: KeyHash;
};

export type UpdateKeyReturnType = ExecuteReturnType;

export type MsgUpdateKey = {
  updateKey: {
    KeyHash: Hex;
    key: "delete" | { insert: Key };
  };
};

export async function updateKey<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: UpdateKeyParameters,
): UpdateKeyReturnType {
  const { keyHash, action, sender } = parameters;

  const getter = getAction(client, getAppConfig, "getAppConfig");

  const { addresses } = await getter<AppConfig>({});

  const UpdateKeyMsg = {
    updateKey: {
      keyHash,
      key: action,
    },
  };

  const typedData: TypedDataParameter = {
    type: [{ name: "update_key", type: "UpdateKey" }],
    extraTypes: {
      UpdateKey: [
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
              // biome-ignore lint/complexity/useLiteralKeys: This is a dynamic type
              { name: Object.keys(action["insert"]).at(0) as string, type: "string" },
            ],
          }),
    },
  };

  return await execute(client, {
    execute: {
      contract: addresses.accountFactory,
      msg: UpdateKeyMsg,
      typedData,
    },
    sender,
  });
}
