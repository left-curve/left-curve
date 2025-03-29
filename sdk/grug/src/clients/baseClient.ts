import type {
  Chain,
  Client,
  ClientConfig,
  ClientExtend,
  Signer,
  Transport,
} from "../types/index.js";
import { uid } from "../utils/uid.js";

export function createBaseClient<
  transport extends Transport = Transport,
  chain extends Chain | undefined = undefined,
  signer extends Signer | undefined = undefined,
  custom extends Record<string, unknown> | undefined = Record<string, unknown> | undefined,
>(parameters: ClientConfig<transport, chain, signer>): Client<transport, chain, signer, custom> {
  const { chain, signer, name = "Base Client", type = "base", ...rest } = parameters;

  const { config: transport, request } = parameters.transport({ chain });

  const client = {
    ...rest,
    signer,
    chain,
    name,
    transport,
    type,
    request,
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

  return Object.assign(client, { extend: extendClient(client) }) as Client<
    transport,
    chain,
    signer,
    custom
  >;
}
