import type { Address, Funds, Transport, TxParameters } from "@left-curve/sdk/types";
import { type ExecuteReturnType, execute } from "../../app/mutations/execute.js";

import type { DangoClient } from "../../../types/clients.js";
import type { ProposalId } from "../../../types/safe.js";
import type { Signer } from "../../../types/signer.js";
import type { TypedDataParameter } from "../../../types/typedData.js";

export type SafeAccountExecuteParameters = {
  sender: Address;
  account: Address;
  proposalId: ProposalId;
  funds?: Funds;
};

export type SafeAccountExecuteReturnType = ExecuteReturnType;

/**
 *  Execute a proposal once it's passed and the timelock (if there is one)
 * has elapsed.
 * @param parameters
 * @param parameters.sender The sender of the vote.
 * @param parameters.account The safe account address.
 * @param parameters.proposalId The proposal ID.
 * @param parameters.execute Whether to execute the proposal immediately.
 * @param txParameters
 * @param txParameters.gasLimit The gas limit for the transaction.
 * @param txParameters.funds The funds to send with the transaction.
 * @returns The tx hash of the transaction.
 */
export async function safeAccountExecute<transport extends Transport>(
  client: DangoClient<transport, Signer>,
  parameters: SafeAccountExecuteParameters,
  txParameters: TxParameters,
): SafeAccountExecuteReturnType {
  const { sender, account, funds, ...executeMsg } = parameters;
  const { gasLimit } = txParameters;

  const msg = { execute: executeMsg };

  const typedData: TypedDataParameter = {
    type: [{ name: "execute", type: "SafeExecute" }],
    extraTypes: {
      SafeExecute: [{ name: "proposalId", type: "uint32" }],
    },
  };

  return await execute(client, {
    sender,
    execute: {
      contract: account,
      msg,
      funds,
      typedData,
    },
    gasLimit,
  });
}
