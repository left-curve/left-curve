import type { Account, Config, ConnectorUId } from "@leftcurve/types";

export type ChangeAccountParameters = {
  account: Account;
  connectorUId: ConnectorUId;
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
