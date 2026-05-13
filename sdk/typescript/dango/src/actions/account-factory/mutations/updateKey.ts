import { getAppConfig } from "#actions/app/queries/getAppConfig.js";
import type { Address, Hex } from "@left-curve/types";
import { type ExecuteReturnType, execute } from "#actions/app/mutations/execute.js";

import type { Client, Key, KeyHash, Signer, TypedDataParameter } from "@left-curve/types";

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

export async function updateKey(
  client: Client<Signer>,
  parameters: UpdateKeyParameters,
): UpdateKeyReturnType {
  const { keyHash, action, sender } = parameters;

  const { addresses } = await getAppConfig(client);

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
