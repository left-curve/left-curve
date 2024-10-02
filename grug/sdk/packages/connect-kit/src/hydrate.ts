import type { Config, State } from "@leftcurve/types";
import { reconnect } from "./actions";

type HydrateParameters = {
  initialState?: State;
  reconnectOnMount?: boolean;
};

export function hydrate(config: Config, parameters: HydrateParameters) {
  const { initialState, reconnectOnMount } = parameters;

  if (initialState && !config.store.persist.hasHydrated())
    config.setState({
      ...initialState,
      chainId: config.chains.some((x) => x.id === initialState.chainId)
        ? initialState.chainId
        : config.chains[0].id,
      connections: reconnectOnMount ? initialState.connections : new Map(),
      status: reconnectOnMount ? "reconnecting" : "disconnected",
    });

  return {
    async onMount() {
      if (config.ssr) {
        await config.store.persist.rehydrate();
      }

      if (reconnectOnMount) {
        reconnect(config);
      } else if (config.storage)
        // Reset connections that may have been hydrated from storage.
        config.setState((x) => ({
          ...x,
          connections: new Map(),
          connectors: new Map(),
          status: "disconnected",
        }));
    },
  };
}
