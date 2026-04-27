import type { Chain } from "../types/chain.js";
import type { Client } from "../types/client.js";
import type { Signer } from "../types/signer.js";
import type { Transport } from "../types/transports.js";

export function getAction<
  transport extends Transport,
  chain extends Chain | undefined,
  signer extends Signer | undefined,
  extended extends { [key: string]: unknown },
  client extends Client<transport, chain, signer, extended>,
  parameters,
  returnType,
>(
  client: client,
  actionFn: (_: any, parameters: parameters) => returnType,
  // Some minifiers drop `Function.prototype.name`, or replace it with short letters,
  // meaning that `actionFn.name` will not always work. For that case, the consumer
  // needs to pass the name explicitly.
  name: string,
): (parameters: parameters) => returnType {
  const action_implicit = client[actionFn.name];
  if (typeof action_implicit === "function")
    return action_implicit as (params: parameters) => returnType;

  const action_explicit = client[name];
  if (typeof action_explicit === "function")
    return action_explicit as (params: parameters) => returnType;

  return (params) => actionFn(client, params);
}
