import type { Client, ClientConfig, ClientExtend } from "@leftcurve/types";
import { uid } from "@leftcurve/utils";

export function createBaseClient(parameters: ClientConfig): Client {
  const { batch, chain, account, key = "base", name = "Base Client", type = "base" } = parameters;

  const { config: transport, query, broadcast } = parameters.transport({ chain });

  const client = {
    account,
    batch,
    chain,
    key,
    name,
    transport,
    query,
    broadcast,
    type,
    uid: uid(),
  };

  function extendClient(base: typeof client) {
    type ExtendFn = (base: typeof client) => unknown;
    return (extendFn: ExtendFn) => {
      const extended = extendFn(base) as ClientExtend;
      for (const key in client) delete extended[key];
      const combined = { ...base, ...extended };
      return Object.assign(combined, { extend: extendClient(combined) });
    };
  }

  return Object.assign(client, { extend: extendClient(client) }) as Client;
}
