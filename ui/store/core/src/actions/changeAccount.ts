import type { Account } from "@left-curve/dango/types";
import type { UID } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";

export type ChangeAccountParameters = {
  account: Account;
  connectorUId: UID;
};

export type ChangeAccountReturnType = void;

export function changeAccount<config extends Config>(
  config: config,
  parameters: ChangeAccountParameters,
): ChangeAccountReturnType {
  const { account, connectorUId } = parameters;

  config.setState((x) => {
    const connection = x.connections.get(connectorUId);
    if (!connection) return x;
    return {
      ...x,
      connections: new Map(x.connections).set(connectorUId, {
        ...connection,
        account,
      }),
    };
  });
}
