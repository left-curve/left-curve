export { createConfig } from "./createConfig";
export { http } from "@leftcurve/sdk";
export { hydrate } from "./hydrate";

/* -------------------------------------------------------------------------- */
/*                                 Connectors                                 */
/* -------------------------------------------------------------------------- */

export { createConnector } from "./connectors/createConnector";
export { passkey } from "./connectors/passkey";
export { eip1193 } from "./connectors/eip1193";

/* -------------------------------------------------------------------------- */
/*                                   Actions                                  */
/* -------------------------------------------------------------------------- */

export {
  type GetChainIdReturnType,
  getChainId,
} from "./actions/getChainId";

export {
  type WatchChainIdParameters,
  type WatchChainIdReturnType,
  watchChainId,
} from "./actions/watchChainId";

export {
  type ConnectParameters,
  type ConnectReturnType,
  type ConnectErrorType,
  connect,
} from "./actions/connect";

export {
  type DisconnectParameters,
  type DisconnectReturnType,
  type DisconnectErrorType,
  disconnect,
} from "./actions/disconnect";

export {
  type GetConnectorsReturnType,
  getConnectors,
} from "./actions/getConnectors";

export {
  type GetAccountReturnType,
  getAccount,
} from "./actions/getAccount";

export {
  type WatchAccountParameters,
  type WatchAccountReturnType,
  watchAccount,
} from "./actions/watchAccount";
