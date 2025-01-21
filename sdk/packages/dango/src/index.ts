export { http } from "@left-curve/sdk";
export { createConfig } from "./store/createConfig.js";
export { hydrate } from "./store/hydrate.js";

/* -------------------------------------------------------------------------- */
/*                                   Chains                                   */
/* -------------------------------------------------------------------------- */

export { devnet } from "@left-curve/sdk/chains";

/* -------------------------------------------------------------------------- */
/*                                   Storage                                  */
/* -------------------------------------------------------------------------- */

export { createMemoryStorage } from "./store/storages/memoryStorage.js";
export { createStorage } from "./store/storages/createStorage.js";

/* -------------------------------------------------------------------------- */
/*                                 Connectors                                 */
/* -------------------------------------------------------------------------- */

export { createConnector } from "./store/connectors/createConnector.js";
export { passkey } from "./store/connectors/passkey.js";
export { eip1193 } from "./store/connectors/eip1193.js";

/* -------------------------------------------------------------------------- */
/*                                   Actions                                  */
/* -------------------------------------------------------------------------- */

export {
  type GetChainIdReturnType,
  getChainId,
} from "./store/actions/getChainId.js";

export {
  type WatchChainIdParameters,
  type WatchChainIdReturnType,
  watchChainId,
} from "./store/actions/watchChainId.js";

export {
  type ConnectParameters,
  type ConnectReturnType,
  type ConnectErrorType,
  connect,
} from "./store/actions/connect.js";

export {
  type DisconnectParameters,
  type DisconnectReturnType,
  type DisconnectErrorType,
  disconnect,
} from "./store/actions/disconnect.js";

export {
  type GetConnectorsReturnType,
  getConnectors,
} from "./store/actions/getConnectors.js";

export {
  type GetAccountReturnType,
  getAccount,
} from "./store/actions/getAccount.js";

export {
  type WatchAccountParameters,
  type WatchAccountReturnType,
  watchAccount,
} from "./store/actions/watchAccount.js";

export {
  type GetBlockExplorerParameters,
  type GetBlockExplorerReturnType,
  type GetBlockExplorerErrorType,
  getBlockExplorer,
} from "./store/actions/getBlockExplorer.js";

export {
  type GetBlockParameters,
  type GetBlockReturnType,
  type GetBlockErrorType,
  getBlock,
} from "./store/actions/getBlock.js";

export {
  type GetPublicClientParameters,
  type GetPublicClientReturnType,
  type GetPublicClientErrorType,
  getPublicClient,
} from "./store/actions/getPublicClient.js";

export {
  type WatchPublicClientParameters,
  type WatchPublicClientReturnType,
  watchPublicClient,
} from "./store/actions/watchPublicClient.js";

export {
  type GetConnectorClientParameters,
  type GetConnectorClientReturnType,
  type GetConnectorClientErrorType,
  getConnectorClient,
} from "./store/actions/getConnectorClient.js";

export {
  type ChangeAccountParameters,
  type ChangeAccountReturnType,
  changeAccount,
} from "./store/actions/changeAccount.js";
