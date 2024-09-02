export { createConfig } from "./createConfig";
export { http } from "@leftcurve/sdk";

/* -------------------------------------------------------------------------- */
/*                                 Connectors                                 */
/* -------------------------------------------------------------------------- */

export { createConnector } from "./connectors/createConnector";
export { passkey } from "./connectors/passkey";

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
