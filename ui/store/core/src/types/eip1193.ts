// This file type is partially forked from viem types in the following repository: https://github.com/wevm/viem/tree/main/src/types
import type {
  Address,
  ExactPartial,
  Hex,
  OneOf,
  Prettify,
  RequestFn,
  RequiredBy,
} from "@left-curve/dango/types";

type Index = `0x${string}`;
type Quantity = `0x${string}`;

type BlockNumber<quantity = bigint> = quantity;

type BlockTag = "latest" | "earliest" | "pending" | "safe" | "finalized";

type BlockIdentifier<quantity = bigint> = {
  /** Whether or not to throw an error if the block is not in the canonical chain as described below. Only allowed in conjunction with the blockHash tag. Defaults to false. */
  requireCanonical?: boolean | undefined;
} & (
  | {
      /** The block in the canonical chain with this number */
      blockNumber: BlockNumber<quantity>;
    }
  | {
      /** The block uniquely identified by this hash. The `blockNumber` and `blockHash` properties are mutually exclusive; exactly one of them must be set. */
      blockHash: Hex;
    }
);

type AccessList = readonly {
  address: Address;
  storageKeys: readonly Hex[];
}[];

type EIP1193Events = {
  on<event extends keyof EIP1193EventMap>(event: event, listener: EIP1193EventMap[event]): void;
  removeListener<event extends keyof EIP1193EventMap>(
    event: event,
    listener: EIP1193EventMap[event],
  ): void;
};

type EIP1193EventMap = {
  accountsChanged(accounts: Address[]): void;
  chainChanged(chainId: string): void;
  connect(connectInfo: ProviderConnectInfo): void;
  disconnect(error: Error): void;
  message(message: ProviderMessage): void;
};

type ProviderConnectInfo = {
  chainId: string;
};

type ProviderMessage = {
  type: string;
  data: unknown;
};

type WalletRpcSchema = [
  /**
   * @description Returns a list of addresses owned by this client
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_accounts' })
   * // => ['0x0fB69...']
   */
  {
    Method: "eth_accounts";
    Parameters?: undefined;
    ReturnType: Address[];
  },
  /**
   * @description Returns the current chain ID associated with the wallet.
   * @example
   * provider.request({ method: 'eth_chainId' })
   * // => '1'
   */
  {
    Method: "eth_chainId";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Estimates the gas necessary to complete a transaction without submitting it to the network
   *
   * @example
   * provider.request({
   *  method: 'eth_estimateGas',
   *  params: [{ from: '0x...', to: '0x...', value: '0x...' }]
   * })
   * // => '0x5208'
   */
  {
    Method: "eth_estimateGas";
    Parameters:
      | [transaction: TransactionRequest]
      | [transaction: TransactionRequest, block: BlockNumber | BlockTag]
      | [
          transaction: TransactionRequest,
          block: BlockNumber | BlockTag,
          stateOverride: RpcStateOverride,
        ];
    ReturnType: Quantity;
  },
  /**
   * @description Requests that the user provides an Ethereum address to be identified by. Typically causes a browser extension popup to appear.
   * @link https://eips.ethereum.org/EIPS/eip-1102
   * @example
   * provider.request({ method: 'eth_requestAccounts' }] })
   * // => ['0x...', '0x...']
   */
  {
    Method: "eth_requestAccounts";
    Parameters?: undefined;
    ReturnType: Address[];
  },
  /**
   * @description Creates, signs, and sends a new transaction to the network
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_sendTransaction', params: [{ from: '0x...', to: '0x...', value: '0x...' }] })
   * // => '0x...'
   */
  {
    Method: "eth_sendTransaction";
    Parameters: [transaction: TransactionRequest];
    ReturnType: Hex;
  },
  /**
   * @description Sends and already-signed transaction to the network
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_sendRawTransaction', params: ['0x...'] })
   * // => '0x...'
   */
  {
    Method: "eth_sendRawTransaction";
    Parameters: [signedTransaction: Hex];
    ReturnType: Hex;
  },
  /**
   * @description Calculates an Ethereum-specific signature in the form of `keccak256("\x19Ethereum Signed Message:\n" + len(message) + message))`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_sign', params: ['0x...', '0x...'] })
   * // => '0x...'
   */
  {
    Method: "eth_sign";
    Parameters: [
      /** Address to use for signing */
      address: Address,
      /** Data to sign */
      data: Hex,
    ];
    ReturnType: Hex;
  },
  /**
   * @description Signs a transaction that can be submitted to the network at a later time using with `eth_sendRawTransaction`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_signTransaction', params: [{ from: '0x...', to: '0x...', value: '0x...' }] })
   * // => '0x...'
   */
  {
    Method: "eth_signTransaction";
    Parameters: [request: TransactionRequest];
    ReturnType: Hex;
  },
  /**
   * @description Calculates an Ethereum-specific signature in the form of `keccak256("\x19Ethereum Signed Message:\n" + len(message) + message))`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_signTypedData_v4', params: [{ from: '0x...', data: [{ type: 'string', name: 'message', value: 'hello world' }] }] })
   * // => '0x...'
   */
  {
    Method: "eth_signTypedData_v4";
    Parameters: [
      /** Address to use for signing */
      address: Address,
      /** Message to sign containing type information, a domain separator, and data */
      message: string,
    ];
    ReturnType: Hex;
  },
  /**
   * @description Returns information about the status of this client’s network synchronization
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_syncing' })
   * // => { startingBlock: '0x...', currentBlock: '0x...', highestBlock: '0x...' }
   */
  {
    Method: "eth_syncing";
    Parameters?: undefined;
    ReturnType: NetworkSync | false;
  },
  /**
   * @description Calculates an Ethereum-specific signature in the form of `keccak256("\x19Ethereum Signed Message:\n" + len(message) + message))`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'personal_sign', params: ['0x...', '0x...'] })
   * // => '0x...'
   */
  {
    Method: "personal_sign";
    Parameters: [
      /** Data to sign */
      data: Hex,
      /** Address to use for signing */
      address: Address,
    ];
    ReturnType: Hex;
  },
  /**
   * @description Add an Ethereum chain to the wallet.
   * @link https://eips.ethereum.org/EIPS/eip-3085
   * @example
   * provider.request({ method: 'wallet_addEthereumChain', params: [{ chainId: 1, rpcUrl: 'https://mainnet.infura.io/v3/...' }] })
   * // => { ... }
   */
  {
    Method: "wallet_addEthereumChain";
    Parameters: [chain: AddEthereumChainParameter];
    ReturnType: null;
  },
  /**
   * @description Returns the status of a call batch that was sent via `wallet_sendCalls`.
   * @link https://eips.ethereum.org/EIPS/eip-5792
   * @example
   * provider.request({ method: 'wallet_getCallsStatus' })
   * // => { ... }
   */
  {
    Method: "wallet_getCallsStatus";
    Parameters?: [string];
    ReturnType: WalletGetCallsStatusReturnType;
  },
  /**
   * @description Gets the connected wallet's capabilities.
   * @link https://eips.ethereum.org/EIPS/eip-5792
   * @example
   * provider.request({ method: 'wallet_getCapabilities' })
   * // => { ... }
   */
  {
    Method: "wallet_getCapabilities";
    Parameters?: [Address];
    ReturnType: Prettify<WalletCapabilitiesRecord>;
  },
  /**
   * @description Gets the wallets current permissions.
   * @link https://eips.ethereum.org/EIPS/eip-2255
   * @example
   * provider.request({ method: 'wallet_getPermissions' })
   * // => { ... }
   */
  {
    Method: "wallet_getPermissions";
    Parameters?: undefined;
    ReturnType: WalletPermission[];
  },
  /**
   * @description Requests permissions from a wallet
   * @link https://eips.ethereum.org/EIPS/eip-7715
   * @example
   * provider.request({ method: 'wallet_grantPermissions', params: [{ ... }] })
   * // => { ... }
   */
  {
    Method: "wallet_grantPermissions";
    Parameters?: [WalletGrantPermissionsParameters];
    ReturnType: Prettify<WalletGrantPermissionsReturnType>;
  },
  /**
   * @description Requests the given permissions from the user.
   * @link https://eips.ethereum.org/EIPS/eip-2255
   * @example
   * provider.request({ method: 'wallet_requestPermissions', params: [{ eth_accounts: {} }] })
   * // => { ... }
   */
  {
    Method: "wallet_requestPermissions";
    Parameters: [permissions: { eth_accounts: Record<string, any> }];
    ReturnType: WalletPermission[];
  },
  /**
   * @description Revokes the given permissions from the user.
   * @link https://github.com/MetaMask/metamask-improvement-proposals/blob/main/MIPs/mip-2.md
   * @example
   * provider.request({ method: 'wallet_revokePermissions', params: [{ eth_accounts: {} }] })
   * // => { ... }
   */
  {
    Method: "wallet_revokePermissions";
    Parameters: [permissions: { eth_accounts: Record<string, any> }];
    ReturnType: null;
  },
  /**
   * @description Requests the connected wallet to send a batch of calls.
   * @link https://eips.ethereum.org/EIPS/eip-5792
   * @example
   * provider.request({ method: 'wallet_sendCalls' })
   * // => { ... }
   */
  {
    Method: "wallet_sendCalls";
    Parameters?: WalletSendCallsParameters;
    ReturnType: string;
  },
  /**
   * @description Requests for the wallet to show information about a call batch
   * that was sent via `wallet_sendCalls`.
   * @link https://eips.ethereum.org/EIPS/eip-5792
   * @example
   * provider.request({ method: 'wallet_showCallsStatus', params: ['...'] })
   */
  {
    Method: "wallet_showCallsStatus";
    Parameters?: [string];
    ReturnType: undefined;
  },
  /**
   * @description Switch the wallet to the given Ethereum chain.
   * @link https://eips.ethereum.org/EIPS/eip-3326
   * @example
   * provider.request({ method: 'wallet_switchEthereumChain', params: [{ chainId: '0xf00' }] })
   * // => { ... }
   */
  {
    Method: "wallet_switchEthereumChain";
    Parameters: [chain: { chainId: string }];
    ReturnType: null;
  },
  /**
   * @description Requests that the user tracks the token in their wallet. Returns a boolean indicating if the token was successfully added.
   * @link https://eips.ethereum.org/EIPS/eip-747
   * @example
   * provider.request({ method: 'wallet_watchAsset' }] })
   * // => true
   */
  {
    Method: "wallet_watchAsset";
    Parameters: WatchAssetParams;
    ReturnType: boolean;
  },
];

type PublicRpcSchema = [
  /**
   * @description Returns the version of the current client
   *
   * @example
   * provider.request({ method: 'web3_clientVersion' })
   * // => 'MetaMask/v1.0.0'
   */
  {
    Method: "web3_clientVersion";
    Parameters?: undefined;
    ReturnType: string;
  },
  /**
   * @description Hashes data using the Keccak-256 algorithm
   *
   * @example
   * provider.request({ method: 'web3_sha3', params: ['0x68656c6c6f20776f726c64'] })
   * // => '0xc94770007dda54cF92009BFF0dE90c06F603a09f'
   */
  {
    Method: "web3_sha3";
    Parameters: [data: Hex];
    ReturnType: string;
  },
  /**
   * @description Determines if this client is listening for new network connections
   *
   * @example
   * provider.request({ method: 'net_listening' })
   * // => true
   */
  {
    Method: "net_listening";
    Parameters?: undefined;
    ReturnType: boolean;
  },
  /**
   * @description Returns the number of peers currently connected to this client
   *
   * @example
   * provider.request({ method: 'net_peerCount' })
   * // => '0x1'
   */
  {
    Method: "net_peerCount";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the chain ID associated with the current network
   *
   * @example
   * provider.request({ method: 'net_version' })
   * // => '1'
   */
  {
    Method: "net_version";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the base fee per blob gas in wei.
   *
   * @example
   * provider.request({ method: 'eth_blobBaseFee' })
   * // => '0x09184e72a000'
   */
  {
    Method: "eth_blobBaseFee";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the number of the most recent block seen by this client
   *
   * @example
   * provider.request({ method: 'eth_blockNumber' })
   * // => '0x1b4'
   */
  {
    Method: "eth_blockNumber";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Executes a new message call immediately without submitting a transaction to the network
   *
   * @example
   * provider.request({ method: 'eth_call', params: [{ to: '0x...', data: '0x...' }] })
   * // => '0x...'
   */
  {
    Method: "eth_call";
    Parameters:
      | [transaction: ExactPartial<TransactionRequest>]
      | [
          transaction: ExactPartial<TransactionRequest>,
          block: BlockNumber | BlockTag | BlockIdentifier,
        ]
      | [
          transaction: ExactPartial<TransactionRequest>,
          block: BlockNumber | BlockTag | BlockIdentifier,
          stateOverrideSet: RpcStateOverride,
        ];
    ReturnType: Hex;
  },
  /**
   * @description Returns the chain ID associated with the current network
   * @example
   * provider.request({ method: 'eth_chainId' })
   * // => '1'
   */
  {
    Method: "eth_chainId";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the client coinbase address.
   * @example
   * provider.request({ method: 'eth_coinbase' })
   * // => '0x...'
   */
  {
    Method: "eth_coinbase";
    Parameters?: undefined;
    ReturnType: Address;
  },
  /**
   * @description Estimates the gas necessary to complete a transaction without submitting it to the network
   *
   * @example
   * provider.request({
   *  method: 'eth_estimateGas',
   *  params: [{ from: '0x...', to: '0x...', value: '0x...' }]
   * })
   * // => '0x5208'
   */
  {
    Method: "eth_estimateGas";
    Parameters:
      | [transaction: TransactionRequest]
      | [transaction: TransactionRequest, block: BlockNumber | BlockTag]
      | [
          transaction: TransactionRequest,
          block: BlockNumber | BlockTag,
          stateOverride: RpcStateOverride,
        ];
    ReturnType: Quantity;
  },
  /**
   * @description Returns a collection of historical gas information
   *
   * @example
   * provider.request({
   *  method: 'eth_feeHistory',
   *  params: ['4', 'latest', ['25', '75']]
   * })
   * // => {
   * //   oldestBlock: '0x1',
   * //   baseFeePerGas: ['0x1', '0x2', '0x3', '0x4'],
   * //   gasUsedRatio: ['0x1', '0x2', '0x3', '0x4'],
   * //   reward: [['0x1', '0x2'], ['0x3', '0x4'], ['0x5', '0x6'], ['0x7', '0x8']]
   * // }
   * */
  {
    Method: "eth_feeHistory";
    Parameters: [
      /** Number of blocks in the requested range. Between 1 and 1024 blocks can be requested in a single query. Less than requested may be returned if not all blocks are available. */
      blockCount: Quantity,
      /** Highest number block of the requested range. */
      newestBlock: BlockNumber | BlockTag,
      /** A monotonically increasing list of percentile values to sample from each block's effective priority fees per gas in ascending order, weighted by gas used. */
      rewardPercentiles: number[] | undefined,
    ];
    ReturnType: FeeHistory;
  },
  /**
   * @description Returns the current price of gas expressed in wei
   *
   * @example
   * provider.request({ method: 'eth_gasPrice' })
   * // => '0x09184e72a000'
   */
  {
    Method: "eth_gasPrice";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the balance of an address in wei
   *
   * @example
   * provider.request({ method: 'eth_getBalance', params: ['0x...', 'latest'] })
   * // => '0x12a05...'
   */
  {
    Method: "eth_getBalance";
    Parameters: [address: Address, block: BlockNumber | BlockTag | BlockIdentifier];
    ReturnType: Quantity;
  },
  /**
   * @description Returns information about a block specified by hash
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getBlockByHash', params: ['0x...', true] })
   * // => {
   * //   number: '0x1b4',
   * //   hash: '0x...',
   * //   parentHash: '0x...',
   * //   ...
   * // }
   */
  {
    Method: "eth_getBlockByHash";
    Parameters: [
      /** hash of a block */
      hash: Hex,
      /** true will pull full transaction objects, false will pull transaction hashes */
      includeTransactionObjects: boolean,
    ];
    ReturnType: Block | null;
  },
  /**
   * @description Returns information about a block specified by number
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getBlockByNumber', params: ['0x1b4', true] })
   * // => {
   * //   number: '0x1b4',
   * //   hash: '0x...',
   * //   parentHash: '0x...',
   * //   ...
   * // }
   */
  {
    Method: "eth_getBlockByNumber";
    Parameters: [
      /** block number, or one of "latest", "safe", "finalized", "earliest" or "pending" */
      block: BlockNumber | BlockTag,
      /** true will pull full transaction objects, false will pull transaction hashes */
      includeTransactionObjects: boolean,
    ];
    ReturnType: Block | null;
  },
  /**
   * @description Returns the number of transactions in a block specified by block hash
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getBlockTransactionCountByHash', params: ['0x...'] })
   * // => '0x1'
   */
  {
    Method: "eth_getBlockTransactionCountByHash";
    Parameters: [hash: Hex];
    ReturnType: Quantity;
  },
  /**
   * @description Returns the number of transactions in a block specified by block number
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getBlockTransactionCountByNumber', params: ['0x1b4'] })
   * // => '0x1'
   */
  {
    Method: "eth_getBlockTransactionCountByNumber";
    Parameters: [block: BlockNumber | BlockTag];
    ReturnType: Quantity;
  },
  /**
   * @description Returns the contract code stored at a given address
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getCode', params: ['0x...', 'latest'] })
   * // => '0x...'
   */
  {
    Method: "eth_getCode";
    Parameters: [address: Address, block: BlockNumber | BlockTag | BlockIdentifier];
    ReturnType: Hex;
  },
  /**
   * @description Returns a list of all logs based on filter ID since the last log retrieval
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getFilterChanges', params: ['0x...'] })
   * // => [{ ... }, { ... }]
   */
  {
    Method: "eth_getFilterChanges";
    Parameters: [filterId: Quantity];
    ReturnType: any[] | Hex[];
  },
  /**
   * @description Returns a list of all logs based on filter ID
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getFilterLogs', params: ['0x...'] })
   * // => [{ ... }, { ... }]
   */
  {
    Method: "eth_getFilterLogs";
    Parameters: [filterId: Quantity];
    ReturnType: any[];
  },
  /**
   * @description Returns a list of all logs based on a filter object
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getLogs', params: [{ fromBlock: '0x...', toBlock: '0x...', address: '0x...', topics: ['0x...'] }] })
   * // => [{ ... }, { ... }]
   */
  {
    Method: "eth_getLogs";
    Parameters: [
      {
        address?: Address | Address[] | undefined;
        topics?: any[] | undefined;
      } & (
        | {
            fromBlock?: BlockNumber | BlockTag | undefined;
            toBlock?: BlockNumber | BlockTag | undefined;
            blockHash?: undefined;
          }
        | {
            fromBlock?: undefined;
            toBlock?: undefined;
            blockHash?: Hex | undefined;
          }
      ),
    ];
    ReturnType: any[];
  },
  /**
   * @description Returns the account and storage values of the specified account including the Merkle-proof.
   * @link https://eips.ethereum.org/EIPS/eip-1186
   * @example
   * provider.request({ method: 'eth_getProof', params: ['0x...', ['0x...'], 'latest'] })
   * // => {
   * //   ...
   * // }
   */
  {
    Method: "eth_getProof";
    Parameters: [
      /** Address of the account. */
      address: Address,
      /** An array of storage-keys that should be proofed and included. */
      storageKeys: Hex[],
      block: BlockNumber | BlockTag,
    ];
    ReturnType: any;
  },
  /**
   * @description Returns the value from a storage position at an address
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getStorageAt', params: ['0x...', '0x...', 'latest'] })
   * // => '0x...'
   */
  {
    Method: "eth_getStorageAt";
    Parameters: [
      address: Address,
      index: Quantity,
      block: BlockNumber | BlockTag | BlockIdentifier,
    ];
    ReturnType: Hex;
  },
  /**
   * @description Returns information about a transaction specified by block hash and transaction index
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getTransactionByBlockHashAndIndex', params: ['0x...', '0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getTransactionByBlockHashAndIndex";
    Parameters: [hash: Hex, index: Quantity];
    ReturnType: TransactionRequest | null;
  },
  /**
   * @description Returns information about a transaction specified by block number and transaction index
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getTransactionByBlockNumberAndIndex', params: ['0x...', '0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getTransactionByBlockNumberAndIndex";
    Parameters: [block: BlockNumber | BlockTag, index: Quantity];
    ReturnType: TransactionRequest | null;
  },
  /**
   * @description Returns information about a transaction specified by hash
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getTransactionByHash', params: ['0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getTransactionByHash";
    Parameters: [hash: Hex];
    ReturnType: TransactionRequest | null;
  },
  /**
   * @description Returns the number of transactions sent from an address
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getTransactionCount', params: ['0x...', 'latest'] })
   * // => '0x1'
   */
  {
    Method: "eth_getTransactionCount";
    Parameters: [address: Address, block: BlockNumber | BlockTag | BlockIdentifier];
    ReturnType: Quantity;
  },
  /**
   * @description Returns the receipt of a transaction specified by hash
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getTransactionReceipt', params: ['0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getTransactionReceipt";
    Parameters: [hash: Hex];
    ReturnType: TransactionReceipt | null;
  },
  /**
   * @description Returns information about an uncle specified by block hash and uncle index position
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getUncleByBlockHashAndIndex', params: ['0x...', '0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getUncleByBlockHashAndIndex";
    Parameters: [hash: Hex, index: Quantity];
    ReturnType: Uncle | null;
  },
  /**
   * @description Returns information about an uncle specified by block number and uncle index position
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getUncleByBlockNumberAndIndex', params: ['0x...', '0x...'] })
   * // => { ... }
   */
  {
    Method: "eth_getUncleByBlockNumberAndIndex";
    Parameters: [block: BlockNumber | BlockTag, index: Quantity];
    ReturnType: Uncle | null;
  },
  /**
   * @description Returns the number of uncles in a block specified by block hash
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getUncleCountByBlockHash', params: ['0x...'] })
   * // => '0x1'
   */
  {
    Method: "eth_getUncleCountByBlockHash";
    Parameters: [hash: Hex];
    ReturnType: Quantity;
  },
  /**
   * @description Returns the number of uncles in a block specified by block number
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_getUncleCountByBlockNumber', params: ['0x...'] })
   * // => '0x1'
   */
  {
    Method: "eth_getUncleCountByBlockNumber";
    Parameters: [block: BlockNumber | BlockTag];
    ReturnType: Quantity;
  },
  /**
   * @description Returns the current maxPriorityFeePerGas in wei.
   * @link https://ethereum.github.io/execution-apis/api-documentation/
   * @example
   * provider.request({ method: 'eth_maxPriorityFeePerGas' })
   * // => '0x5f5e100'
   */
  {
    Method: "eth_maxPriorityFeePerGas";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Creates a filter to listen for new blocks that can be used with `eth_getFilterChanges`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_newBlockFilter' })
   * // => '0x1'
   */
  {
    Method: "eth_newBlockFilter";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Creates a filter to listen for specific state changes that can then be used with `eth_getFilterChanges`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_newFilter', params: [{ fromBlock: '0x...', toBlock: '0x...', address: '0x...', topics: ['0x...'] }] })
   * // => '0x1'
   */
  {
    Method: "eth_newFilter";
    Parameters: [
      filter: {
        fromBlock?: BlockNumber | BlockTag | undefined;
        toBlock?: BlockNumber | BlockTag | undefined;
        address?: Address | Address[] | undefined;
        topics?: undefined;
      },
    ];
    ReturnType: Quantity;
  },
  /**
   * @description Creates a filter to listen for new pending transactions that can be used with `eth_getFilterChanges`
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_newPendingTransactionFilter' })
   * // => '0x1'
   */
  {
    Method: "eth_newPendingTransactionFilter";
    Parameters?: undefined;
    ReturnType: Quantity;
  },
  /**
   * @description Returns the current Ethereum protocol version
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_protocolVersion' })
   * // => '54'
   */
  {
    Method: "eth_protocolVersion";
    Parameters?: undefined;
    ReturnType: string;
  },
  /**
   * @description Sends a **signed** transaction to the network
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_sendRawTransaction', params: ['0x...'] })
   * // => '0x...'
   */
  {
    Method: "eth_sendRawTransaction";
    Parameters: [signedTransaction: Hex];
    ReturnType: Hex;
  },
  /**
   * @description Destroys a filter based on filter ID
   * @link https://eips.ethereum.org/EIPS/eip-1474
   * @example
   * provider.request({ method: 'eth_uninstallFilter', params: ['0x1'] })
   * // => true
   */
  {
    Method: "eth_uninstallFilter";
    Parameters: [filterId: Quantity];
    ReturnType: boolean;
  },
];

type WatchAssetParams = {
  /** Token type. */
  type: "ERC20";
  options: {
    /** The address of the token contract */
    address: string;
    /** A ticker symbol or shorthand, up to 11 characters */
    symbol: string;
    /** The number of token decimals */
    decimals: number;
    /** A string url of the token logo */
    image?: string | undefined;
  };
};

type NetworkSync = {
  /** The current block number */
  currentBlock: Quantity;
  /** Number of latest block on the network */
  highestBlock: Quantity;
  /** Block number at which syncing started */
  startingBlock: Quantity;
};

type AddEthereumChainParameter = {
  /** A 0x-prefixed hexadecimal string */
  chainId: string;
  /** The chain name. */
  chainName: string;
  /** Native currency for the chain. */
  nativeCurrency?:
    | {
        name: string;
        symbol: string;
        decimals: number;
      }
    | undefined;
  rpcUrls: readonly string[];
  blockExplorerUrls?: string[] | undefined;
  iconUrls?: string[] | undefined;
};

type WalletCapabilities = {
  [capability: string]: any;
};

type WalletSendCallsParameters<
  capabilities extends WalletCapabilities = WalletCapabilities,
  chainId extends Hex | number = Hex,
  quantity extends Quantity | bigint = Quantity,
> = [
  {
    calls: readonly {
      chainId?: chainId | undefined;
      to?: Address | undefined;
      data?: Hex | undefined;
      value?: quantity | undefined;
    }[];
    capabilities?: capabilities | undefined;
    /** @deprecated Use `chainId` on `calls` instead. */
    chainId?: chainId | undefined;
    from: Address;
    version: string;
  },
];

type WalletPermissionCaveat = {
  type: string;
  value: any;
};

type WalletPermission = {
  caveats: WalletPermissionCaveat[];
  date: number;
  id: string;
  invoker: `http://${string}` | `https://${string}`;
  parentCapability: "eth_accounts" | string;
};

type WalletGrantPermissionsParameters = {
  signer?:
    | {
        type: string;
        data?: unknown | undefined;
      }
    | undefined;
  permissions: readonly {
    data: unknown;
    policies: readonly {
      data: unknown;
      type: string;
    }[];
    required?: boolean | undefined;
    type: string;
  }[];
  expiry: number;
};

type WalletGrantPermissionsReturnType = {
  expiry: number;
  factory?: `0x${string}` | undefined;
  factoryData?: string | undefined;
  grantedPermissions: readonly {
    data: unknown;
    policies: readonly {
      data: unknown;
      type: string;
    }[];
    required?: boolean | undefined;
    type: string;
  }[];
  permissionsContext: string;
  signerData?:
    | {
        userOpBuilder?: `0x${string}` | undefined;
        submitToAddress?: `0x${string}` | undefined;
      }
    | undefined;
};

type WalletCapabilitiesRecord<
  capabilities extends WalletCapabilities = WalletCapabilities,
  id extends string | number = Hex,
> = {
  [chainId in id]: capabilities;
};

type WalletGetCallsStatusReturnType<quantity = Hex, status = Hex> = {
  status: "PENDING" | "CONFIRMED";
  receipts?: WalletCallReceipt<quantity, status>[] | undefined;
};

type WalletCallReceipt<quantity = Hex, status = Hex> = {
  logs: {
    address: Hex;
    data: Hex;
    topics: Hex[];
  }[];
  status: status;
  blockHash: Hex;
  blockNumber: quantity;
  gasUsed: quantity;
  transactionHash: Hex;
};

type RpcAccountStateOverride = {
  /** Fake balance to set for the account before executing the call. <32 bytes */
  balance?: Hex | undefined;
  /** Fake nonce to set for the account before executing the call. <8 bytes */
  nonce?: Hex | undefined;
  /** Fake EVM bytecode to inject into the account before executing the call. */
  code?: Hex | undefined;
  /** Fake key-value mapping to override all slots in the account storage before executing the call. */
  state?: RpcStateMapping | undefined;
  /** Fake key-value mapping to override individual slots in the account storage before executing the call. */
  stateDiff?: RpcStateMapping | undefined;
};

type RpcStateOverride = {
  [address: Address]: RpcAccountStateOverride;
};

/** A key-value mapping of slot and storage values (supposedly 32 bytes each) */
type RpcStateMapping = {
  [slots: Hex]: Hex;
};

type TransactionRequestBase<quantity = bigint, index = number, type = string> = {
  /** Contract code or a hashed method call with encoded args */
  data?: Hex | undefined;
  /** Transaction sender */
  from: Address;
  /** Gas provided for transaction execution */
  gas?: quantity | undefined;
  /** Unique number identifying this transaction */
  nonce?: index | undefined;
  /** Transaction recipient */
  to?: Address | null | undefined;
  /** Transaction type */
  type?: type | undefined;
  /** Value in wei sent with this transaction */
  value?: quantity | undefined;
};

type TransactionRequest = OneOf<
  | TransactionRequestLegacy<Quantity, Index, "0x0">
  | TransactionRequestEIP2930<Quantity, Index, "0x1">
  | TransactionRequestEIP1559<Quantity, Index, "0x2">
  | TransactionRequestEIP4844<Quantity, Index, "0x3">
>;

type TransactionRequestLegacy<
  quantity = bigint,
  index = number,
  type = "legacy",
> = TransactionRequestBase<quantity, index, type> & ExactPartial<FeeValuesLegacy<quantity>>;

type TransactionRequestEIP2930<
  quantity = bigint,
  index = number,
  type = "eip2930",
> = TransactionRequestBase<quantity, index, type> &
  ExactPartial<FeeValuesLegacy<quantity>> & {
    accessList?: AccessList | undefined;
  };

type TransactionRequestEIP1559<
  quantity = bigint,
  index = number,
  type = "eip1559",
> = TransactionRequestBase<quantity, index, type> &
  ExactPartial<FeeValuesEIP1559<quantity>> & {
    accessList?: AccessList | undefined;
  };

type TransactionRequestEIP4844<quantity = bigint, index = number, type = "eip4844"> = RequiredBy<
  TransactionRequestBase<quantity, index, type>,
  "to"
> &
  RequiredBy<ExactPartial<FeeValuesEIP4844<quantity>>, "maxFeePerBlobGas"> & {
    accessList?: AccessList | undefined;
    /** The blobs associated with this transaction. */
    blobs: readonly Hex[] | readonly Uint8Array[];
    blobVersionedHashes?: readonly Hex[] | undefined;
    kzg?: Kzg | undefined;
    sidecars?: readonly BlobSidecar<Hex>[] | undefined;
  };

type FeeValuesLegacy<quantity = bigint> = {
  /** Base fee per gas. */
  gasPrice: quantity;
  maxFeePerBlobGas?: undefined;
  maxFeePerGas?: undefined;
  maxPriorityFeePerGas?: undefined;
};

type FeeHistory<quantity = bigint> = {
  /**
   * An array of block base fees per gas (in wei). This includes the next block after
   * the newest of the returned range, because this value can be derived from the newest block.
   * Zeroes are returned for pre-EIP-1559 blocks. */
  baseFeePerGas: quantity[];
  /** An array of block gas used ratios. These are calculated as the ratio of gasUsed and gasLimit. */
  gasUsedRatio: number[];
  /** Lowest number block of the returned range. */
  oldestBlock: quantity;
  /** An array of effective priority fees (in wei) per gas data points from a single block. All zeroes are returned if the block is empty. */
  reward?: quantity[][] | undefined;
};

type FeeValuesEIP1559<quantity = bigint> = {
  gasPrice?: undefined;
  maxFeePerBlobGas?: undefined;
  /** Total fee per gas in wei (gasPrice/baseFeePerGas + maxPriorityFeePerGas). */
  maxFeePerGas: quantity;
  /** Max priority fee per gas (in wei). */
  maxPriorityFeePerGas: quantity;
};

type FeeValuesEIP4844<quantity = bigint> = {
  gasPrice?: undefined;
  /** Maximum total fee per gas sender is willing to pay for blob gas (in wei). */
  maxFeePerBlobGas: quantity;
  /** Total fee per gas in wei (gasPrice/baseFeePerGas + maxPriorityFeePerGas). */
  maxFeePerGas: quantity;
  /** Max priority fee per gas (in wei). */
  maxPriorityFeePerGas: quantity;
};

type Kzg = {
  /**
   * Convert a blob to a KZG commitment.
   */
  blobToKzgCommitment(blob: Uint8Array): Uint8Array;
  /**
   * Given a blob, return the KZG proof that is used to verify it against the
   * commitment.
   */
  computeBlobKzgProof(blob: Uint8Array, commitment: Uint8Array): Uint8Array;
};

type BlobSidecar<type extends Hex | Uint8Array = Hex | Uint8Array> = {
  /** The blob associated with the transaction. */
  blob: type;
  /** The KZG commitment corresponding to this blob. */
  commitment: type;
  /** The KZG proof corresponding to this blob and commitment. */
  proof: type;
};

type EIP1474Methods = [...PublicRpcSchema, ...WalletRpcSchema];

export type EIP1193Provider = Prettify<
  EIP1193Events & {
    request: RequestFn<EIP1474Methods>;
  }
>;

type Block<
  quantity = bigint,
  includeTransactions extends boolean = boolean,
  blockTag extends BlockTag = BlockTag,
  transaction = TransactionRequest,
> = {
  /** Base fee per gas */
  baseFeePerGas: quantity | null;
  /** Total used blob gas by all transactions in this block */
  blobGasUsed: quantity;
  /** Difficulty for this block */
  difficulty: quantity;
  /** Excess blob gas */
  excessBlobGas: quantity;
  /** "Extra data" field of this block */
  extraData: Hex;
  /** Maximum gas allowed in this block */
  gasLimit: quantity;
  /** Total used gas by all transactions in this block */
  gasUsed: quantity;
  /** Block hash or `null` if pending */
  hash: blockTag extends "pending" ? null : Hex;
  /** Logs bloom filter or `null` if pending */
  logsBloom: blockTag extends "pending" ? null : Hex;
  /** Address that received this block’s mining rewards */
  miner: Address;
  /** Unique identifier for the block. */
  mixHash: Hex;
  /** Proof-of-work hash or `null` if pending */
  nonce: blockTag extends "pending" ? null : Hex;
  /** Block number or `null` if pending */
  number: blockTag extends "pending" ? null : quantity;
  /** Parent block hash */
  parentHash: Hex;
  /** Root of the this block’s receipts trie */
  receiptsRoot: Hex;
  sealFields: Hex[];
  /** SHA3 of the uncles data in this block */
  sha3Uncles: Hex;
  /** Size of this block in bytes */
  size: quantity;
  /** Root of this block’s final state trie */
  stateRoot: Hex;
  /** Unix timestamp of when this block was collated */
  timestamp: quantity;
  /** Total difficulty of the chain until this block */
  totalDifficulty: quantity | null;
  /** List of transaction objects or hashes */
  transactions: includeTransactions extends true ? transaction[] : Hex[];
  /** Root of this block’s transaction trie */
  transactionsRoot: Hex;
  /** List of uncle hashes */
  uncles: Hex[];
  /** List of withdrawal objects */
  withdrawals?: Withdrawal[] | undefined;
  /** Root of the this block’s withdrawals trie */
  withdrawalsRoot?: Hex | undefined;
};

type Withdrawal = {
  address: Hex;
  amount: Hex;
  index: Hex;
  validatorIndex: Hex;
};

type TransactionReceipt<quantity = bigint, index = number, status = "success" | "reverted"> = {
  /** The actual value per gas deducted from the sender's account for blob gas. Only specified for blob transactions as defined by EIP-4844. */
  blobGasPrice?: quantity | undefined;
  /** The amount of blob gas used. Only specified for blob transactions as defined by EIP-4844. */
  blobGasUsed?: quantity | undefined;
  /** Hash of block containing this transaction */
  blockHash: Hex;
  /** Number of block containing this transaction */
  blockNumber: quantity;
  /** Address of new contract or `null` if no contract was created */
  contractAddress: Address | null | undefined;
  /** Gas used by this and all preceding transactions in this block */
  cumulativeGasUsed: quantity;
  /** Pre-London, it is equal to the transaction's gasPrice. Post-London, it is equal to the actual gas price paid for inclusion. */
  effectiveGasPrice: quantity;
  /** Transaction sender */
  from: Address;
  /** Gas used by this transaction */
  gasUsed: quantity;
  /** List of log objects generated by this transaction */
  logs: any;
  /** Logs bloom filter */
  logsBloom: Hex;
  /** The post-transaction state root. Only specified for transactions included before the Byzantium upgrade. */
  root?: Hex | undefined;
  /** `success` if this transaction was successful or `reverted` if it failed */
  status: status;
  /** Transaction recipient or `null` if deploying a contract */
  to: Address | null;
  /** Hash of this transaction */
  transactionHash: Hex;
  /** Index of this transaction in the block */
  transactionIndex: index;
  /** Transaction type */
  type: "eip1559" | "eip2930" | "eip4844" | "legacy";
};

type Uncle<
  quantity = bigint,
  includeTransactions extends boolean = boolean,
  blockTag extends BlockTag = BlockTag,
  transaction = TransactionRequest,
> = Block<quantity, includeTransactions, blockTag, transaction>;
