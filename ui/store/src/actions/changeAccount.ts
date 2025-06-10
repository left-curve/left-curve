import type { Account, Address } from "@left-curve/dango/types";
import type { UID } from "@left-curve/dango/types";

import type { Config } from "../types/store.js";

export type ChangeAccountParameters = {
  address: Address;
  connectorUId?: UID;
};

export type ChangeAccountReturnType = void;

export function changeAccount<config extends Config>(
  config: config,
  parameters: ChangeAccountParameters,
): ChangeAccountReturnType {
  const { address, connectorUId } = parameters;

  config.setState((x) => {
    const Uid = connectorUId || config.state.current;
    const connection = x.connectors.get(Uid || "");
    if (!connection) return x;

    const account = connection.accounts.find((account) => account.address === address);

    if (!account) return x;

    return {
      ...x,
      connectors: new Map(x.connectors).set(Uid as string, {
        ...connection,
        account,
      }),
    };
  });
}
