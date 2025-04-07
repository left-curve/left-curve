import { reconnect } from "./actions/reconnect.js";
import { eip6963 } from "./connectors/eip6963.js";
import { ConnectionStatus } from "./types/store.js";

import type { Config, State } from "./types/store.js";

type HydrateParameters = {
  initialState?: State;
  reconnectOnMount?: boolean;
};

export function rehydrate(config: Config, parameters: HydrateParameters) {
  const { initialState, reconnectOnMount } = parameters;

  if (initialState && !config._internal.store.persist.hasHydrated())
    config.setState({
      ...initialState,
      chainId: config.chain.id,
      connectors: reconnectOnMount ? initialState.connectors : new Map(),
      status: reconnectOnMount ? ConnectionStatus.Reconnecting : ConnectionStatus.Disconnected,
    });

  return {
    async onMount() {
      if (config._internal.ssr) {
        // await config._internal.store.persist.rehydrate();
        if (config._internal.mipd) {
          config._internal.connectors.setState((connectors) => {
            const rdnsSet = new Set<string>();
            for (const connector of connectors ?? []) {
              if (connector.rdns) rdnsSet.add(connector.rdns);
            }
            const mipdConnectors = [];
            const providers = config._internal.mipd?.getProviders() ?? [];
            for (const provider of providers) {
              if (rdnsSet.has(provider.info.rdns)) continue;
              const connectorFn = eip6963(provider);
              const connector = config._internal.connectors.setup(connectorFn);
              mipdConnectors.push(connector);
            }
            return [...connectors, ...mipdConnectors];
          });
        }
      }

      if (reconnectOnMount) {
        config.subscribe(
          (x) => x.isMipdLoaded,
          (isMipdLoaded) => {
            if (isMipdLoaded) reconnect(config);
          },
        );
      } else if (config.storage)
        // Reset connections that may have been hydrated from storage.
        config.setState((x) => ({
          ...x,
          connectors: new Map(),
          status: ConnectionStatus.Disconnected,
        }));
    },
  };
}
