import type {
  Address,
  Chain,
  Client,
  Funds,
  Json,
  Signer,
  Transport,
  TxMessageType,
  TypedDataParameter,
} from "@leftcurve/types";
import { getCoinsTypedData } from "@leftcurve/utils";
import { type SignAndBroadcastTxReturnType, signAndBroadcastTx } from "./signAndBroadcastTx";

export type ExecuteParameters = {
  sender: Address;
  contract: Address;
  msg: Json;
  funds?: Funds;
  gasLimit?: number;
  typedData?: TypedDataParameter;
};

export type ExecuteReturnType = SignAndBroadcastTxReturnType;

export async function execute<chain extends Chain | undefined, signer extends Signer>(
  client: Client<Transport, chain, signer>,
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
