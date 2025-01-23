import type {
  Address,
  Funds,
  Json,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@left-curve/types";
import { getCoinsTypedData } from "../../../utils/typedData.js";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx.js";

import type { DangoClient, Signer } from "../../../types/index.js";

export type ExecuteParameters = {
  sender: Address;
  contract: Address;
  msg: Json;
  funds?: Funds;
  gasLimit?: number;
  typedData?: TypedDataParameter;
};

export type ExecuteReturnType = SignAndBroadcastTxReturnType;

export async function execute<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: ExecuteParameters,
): ExecuteReturnType {
  const { sender, contract, msg, gasLimit, funds = {} } = parameters;
  const executeMsg = {
    execute: {
      contract,
      msg,
      funds,
    },
  };

  const { extraTypes = {}, type = [] } = parameters.typedData || {};

  const typedData: TypedDataParameter<TxMessageType> = {
    type: [{ name: "execute", type: "Execute" }],
    extraTypes: {
      Execute: [
        { name: "contract", type: "address" },
        { name: "msg", type: "ExecuteMessage" },
        { name: "funds", type: "Funds" },
      ],
      Funds: [...getCoinsTypedData(funds)],
      ExecuteMessage: type,
      ...extraTypes,
    },
  };

  return await signAndBroadcastTx(client, { sender, messages: [executeMsg], typedData, gasLimit });
}
