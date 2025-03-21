export { createConfig } from "./createConfig.js";

export {
  DangoStoreContext,
  DangoStoreProvider,
  type DangoStoreProviderProps,
} from "./context.js";

export { devnet, http, graphql } from "@left-curve/dango";

/* -------------------------------------------------------------------------- */
/*                                    Hooks                                   */
/* -------------------------------------------------------------------------- */

export {
  type UseConfigParameters,
  type UseConfigReturnType,
  useConfig,
} from "./hooks/useConfig.js";

export {
  type UseChainIdParameters,
  type UseChainIdReturnType,
  useChainId,
} from "./hooks/useChainId.js";

export {
  type UseConnectParameters,
  type UseConnectReturnType,
  useConnect,
} from "./hooks/useConnect.js";

export {
  type UseConnectorsParameters,
  type UseConnectorsReturnType,
  useConnectors,
} from "./hooks/useConnectors.js";

export {
  type UseAccountParameters,
  type UseAccountReturnType,
  useAccount,
} from "./hooks/useAccount.js";

export {
  type UseBlockParameters,
  type UseBlockReturnType,
  useBlock,
} from "./hooks/useBlock.js";

export {
  type UsePublicClientParameters,
  type UsePublicClientReturnType,
  usePublicClient,
} from "./hooks/usePublicClient.js";

export {
  type UseDisconnectParameters,
  type UseDisconnectReturnType,
  useDisconnect,
} from "./hooks/useDisconnect.js";

export {
  type UsePricesParameters,
  usePrices,
} from "./hooks/usePrices.js";

export {
  type UseBalancesParameters,
  type UseBalancesReturnType,
  useBalances,
} from "./hooks/useBalances.js";

export {
  type UseStorageOptions,
  useStorage,
} from "./hooks/useStorage.js";

export {
  type UseConnectorClientParameters,
  type UseConnectorClientReturnType,
  useConnectorClient,
} from "./hooks/useConnectorClient.js";

export {
  type UseAccountInfoParameters,
  type UseAccountInfoReturnType,
  useAccountInfo,
} from "./hooks/useAccountInfo.js";

export {
  type UseDataChannelParamaters,
  type UseDataChannelReturnType,
  useDataChannel,
} from "./hooks/useDataChannel.js";

export {
  type UseLoginWithDesktopParameters,
  type UseLoginWithDesktopReturnType,
  useLoginWithDesktop,
} from "./hooks/useLoginWithDesktop.js";

export {
  type UseLoginParameters,
  type UseLoginReturnType,
  useLogin,
} from "./hooks/useLogin.js";

export { useSessionKey } from "./hooks/useSessionKey.js";

export { useSigningClient } from "./hooks/useSigningClient.js";

export { rehydrate } from "./rehydrate.js";

/* -------------------------------------------------------------------------- */
/*                                   Storage                                  */
/* -------------------------------------------------------------------------- */

export { createMemoryStorage } from "./storages/memoryStorage.js";
export { createStorage } from "./storages/createStorage.js";

/* -------------------------------------------------------------------------- */
/*                                 Connectors                                 */
/* -------------------------------------------------------------------------- */

export { createConnector } from "./connectors/createConnector.js";
export { passkey } from "./connectors/passkey.js";
export { eip1193 } from "./connectors/eip1193.js";
export { eip6963 } from "./connectors/eip6963.js";
export { session } from "./connectors/session.js";

/* -------------------------------------------------------------------------- */
/*                                   Actions                                  */
/* -------------------------------------------------------------------------- */

export {
  type GetChainIdReturnType,
  getChainId,
} from "./actions/getChainId.js";

export {
  type WatchChainIdParameters,
  type WatchChainIdReturnType,
  watchChainId,
} from "./actions/watchChainId.js";

export {
  type ConnectParameters,
  type ConnectReturnType,
  type ConnectErrorType,
  connect,
} from "./actions/connect.js";

export {
  type DisconnectParameters,
  type DisconnectReturnType,
  type DisconnectErrorType,
  disconnect,
} from "./actions/disconnect.js";

export {
  type GetConnectorsReturnType,
  getConnectors,
} from "./actions/getConnectors.js";

export {
  type GetAccountReturnType,
  getAccount,
} from "./actions/getAccount.js";

export {
  type WatchAccountParameters,
  type WatchAccountReturnType,
  watchAccount,
} from "./actions/watchAccount.js";

export {
  type GetBlockParameters,
  type GetBlockReturnType,
  type GetBlockErrorType,
  getBlock,
} from "./actions/getBlock.js";

export {
  type GetPublicClientParameters,
  type GetPublicClientReturnType,
  type GetPublicClientErrorType,
  getPublicClient,
} from "./actions/getPublicClient.js";

export {
  type WatchPublicClientParameters,
  type WatchPublicClientReturnType,
  watchPublicClient,
} from "./actions/watchPublicClient.js";

export {
  type GetConnectorClientParameters,
  type GetConnectorClientReturnType,
  type GetConnectorClientErrorType,
  getConnectorClient,
} from "./actions/getConnectorClient.js";

export {
  type ChangeAccountParameters,
  type ChangeAccountReturnType,
  changeAccount,
} from "./actions/changeAccount.js";

/* -------------------------------------------------------------------------- */
/*                                  Handlers                                  */
/* -------------------------------------------------------------------------- */

export {
  type ConnectData,
  type ConnectVariables,
  type ConnectMutate,
  type ConnectMutateAsync,
  connectMutationOptions,
} from "./handlers/connect.js";

export {
  type DisconnectData,
  type DisconnectVariables,
  type DisconnectMutate,
  type DisconnectMutateAsync,
  disconnectMutationOptions,
} from "./handlers/disconnect.js";

export {
  type GetBlockData,
  type GetBlockQueryFnData,
  type GetBlockQueryKey,
  type GetBlockOptions,
  getBlockQueryOptions,
  getBlockQueryKey,
} from "./handlers/getBlock.js";

export {
  type GetBalancesData,
  type GetBalancesQueryFnData,
  type GetBalancesQueryKey,
  type GetBalancesOptions,
  type GetBalancesErrorType,
  getBalancesQueryOptions,
  getBalancesQueryKey,
} from "./handlers/getBalances.js";

export {
  type GetConnectorClientData,
  type GetConnectorClientFnData,
  type GetConnectorClientQueryKey,
  type GetConnectorClientOptions,
  getConnectorClientQueryOptions,
  getConnectorClientQueryKey,
} from "./handlers/getConnectorClient.js";

export {
  type GetAccountInfoData,
  type GetAccountInfoQueryFnData,
  type GetAccountInfoQueryKey,
  type GetAccountInfoOptions,
  type GetAccountInfoErrorType,
  getAccountInfoQueryOptions,
  getAccountInfoQueryKey,
} from "./handlers/getAccountInfo.js";
