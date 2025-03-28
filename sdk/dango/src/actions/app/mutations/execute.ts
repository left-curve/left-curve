import type { Address, Funds, Json, Transport } from "@left-curve/sdk/types";
import { getCoinsTypedData } from "../../../utils/typedData.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type {
  DangoClient,
  Signer,
  TxMessageType,
  TypedDataParameter,
} from "../../../types/index.js";

export type ExecuteParameters = {
  sender: Address;
  execute: ExecuteMsg | ExecuteMsg[];
  gasLimit?: number;
};

export type ExecuteMsg = {
  contract: Address;
  msg: Json;
  typedData?: TypedDataParameter;
  funds?: Funds;
};

export type ExecuteReturnType = SignAndBroadcastTxReturnType;

export async function execute<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: ExecuteParameters,
): ExecuteReturnType {
  const { sender, gasLimit, execute } = parameters;

  const executeMsgs = Array.isArray(execute) ? execute : [execute];

  const { messages, typedData } = executeMsgs.reduce(
    (acc, { contract, msg, funds = {}, typedData }, index) => {
      acc.messages.push({
        execute: {
          contract,
          msg,
          funds,
        },
      });

      const { extraTypes = {}, type = [] } = typedData || {};

      acc.typedData.type.push({
        name: "execute",
        type: `Execute${index}`,
      } as unknown as TxMessageType);
      acc.typedData.extraTypes[`Execute${index}`] = [
        { name: "contract", type: "address" },
        { name: "msg", type: `ExecuteMessage${index}` },
        { name: "funds", type: `Funds${index}` },
      ];
      acc.typedData.extraTypes[`ExecuteMessage${index}`] = type;
      acc.typedData.extraTypes[`Funds${index}`] = [...getCoinsTypedData(funds)];
      acc.typedData.extraTypes = { ...acc.typedData.extraTypes, ...extraTypes };

      return acc;
    },
    {
      messages: [],
      typedData: {
        type: [],
        extraTypes: {},
      },
    } as {
      messages: { execute: Omit<ExecuteMsg, "typedData"> }[];
      typedData: TypedDataParameter<TxMessageType>;
    },
  );

  return await signAndBroadcastTx(client, { sender, messages, typedData, gasLimit });
}
