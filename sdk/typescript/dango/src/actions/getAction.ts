import type { Client } from "../types/client.js";

export function getAction<parameters, returnType>(
  client: Client,
  actionFn: (_: any, parameters: parameters) => returnType,
  name: string,
): (parameters: parameters) => returnType {
  const action_implicit = (client as any)[actionFn.name];
  if (typeof action_implicit === "function")
    return action_implicit as (params: parameters) => returnType;

  const action_explicit = (client as any)[name];
  if (typeof action_explicit === "function")
    return action_explicit as (params: parameters) => returnType;

  return (params) => actionFn(client, params);
}
