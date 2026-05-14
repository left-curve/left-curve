import type { Client, ClientConfig, ClientExtend, Signer } from "@left-curve/types";
import { uid } from "@left-curve/utils";

export function createBaseClient<
  signer extends Signer | undefined = undefined,
  custom extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
>(parameters: ClientConfig<signer>): Client<signer, custom> {
  const {
    signer,
    name = "Base Client",
    type = "base",
    chain,
    transport: _transport,
    ...rest
  } = parameters;

  const { request, subscribe } = parameters.transport(chain);

  const client = {
    ...rest,
    signer,
    chain,
    name,
    type,
    request,
    subscribe,
    uid: uid(),
  };

  function extendClient(base: typeof client) {
    type ExtendFn = (base: typeof client) => unknown;
    return (extendFn: ExtendFn) => {
      const extended = extendFn(base) as ClientExtend;
      for (const key in client) delete extended[key];
      const combined = Object.assign(base, extended);
      return Object.assign(combined, { extend: extendClient(combined) });
    };
  }

  return Object.assign(client, { extend: extendClient(client) }) as Client<signer, custom>;
}
