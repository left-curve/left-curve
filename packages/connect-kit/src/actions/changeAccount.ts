import type { Account, Config, ConnectorId } from "@leftcurve/types";

export type ChangeAccountParameters = {
  account: Account;
  connectorId: ConnectorId;
};

export type ChangeAccountReturnType = void;

export function changeAccount<config extends Config>(
  config: config,
  parameters: ChangeAccountParameters,
): ChangeAccountReturnType {
  const { account, connectorId } = parameters;

  config.setState((x) => {
    const connection = x.connections.get(connectorId);
    if (!connection) return x;
    return {
      ...x,
      connections: new Map(x.connections).set(connectorId, {
        ...connection,
        account,
      }),
    };
  });
}
