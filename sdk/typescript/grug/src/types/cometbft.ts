import type { Base64, Hex } from "./encoding.js";
import type { Prettify } from "./utils.js";

export type CometBftRpcSchema = [
  {
    Method: "abci_info";
    Parameters?: undefined;
    ReturnType: {
      readonly response: {
        readonly data?: string;
        readonly last_block_height?: string;
        readonly last_block_app_hash?: Base64;
      };
    };
  },
  {
    Method: "abci_query";
    Parameters: {
      readonly path: string;
      readonly data: Hex;
      readonly height?: string;
      /**
       * A flag that defines if proofs are included in the response or not.
       *
       * Internally this is mapped to the old inverse name `trusted` for Tendermint < 0.26.
       * Starting with Tendermint 0.26, the default value changed from true to false.
       */
      readonly prove?: boolean;
    };
    ReturnType: {
      response: QueryAbciResponse;
    };
  },
  {
    Method: "block";
    Parameters: {
      readonly height?: string;
    };
    ReturnType: {
      readonly blockId: BlockId;
      readonly block: Block;
    };
  },
  {
    Method: "blockchain";
    Parameters: {
      readonly minHeight?: string;
      readonly maxHeight?: string;
    };
    ReturnType: {
      readonly last_height: string;
      readonly block_metas: readonly BlockMeta[];
    };
  },
  {
    Method: "block_results";
    Parameters: {
      readonly height?: string;
    };
    ReturnType: {
      readonly height: string;
      readonly txs_results: readonly TxData[] | null;
      readonly begin_block_events: readonly TxEvent[] | null;
      readonly end_block_events: readonly TxEvent[] | null;
      readonly validator_updates: readonly ValidatorUpdate[] | null;
      readonly consensus_param_updates: ConsensusParams | null;
    };
  },
  {
    Method: "block_search";
    Parameters: {
      readonly query: string;
      readonly page?: string;
      readonly per_page?: string;
      readonly order_by?: string;
    };
    ReturnType: {
      readonly blocks: readonly {
        readonly block_id: BlockId;
        readonly block: Block;
      }[];
      readonly total_count: string;
    };
  },
  {
    Method: "broadcast_tx_async";
    Parameters: {
      readonly tx: Base64;
    };
    ReturnType: {
      readonly hash: Hex;
    };
  },
  {
    Method: "broadcast_tx_sync";
    Parameters: {
      readonly tx: Base64;
    };
    ReturnType: Prettify<
      TxData & {
        readonly hash: Uint8Array;
      }
    >;
  },
  {
    Method: "broadcast_tx_commit";
    Parameters: {
      readonly tx: Base64;
    };
    ReturnType: {
      readonly height: string;
      readonly hash: Hex;
      readonly check_tx: TxData;
      readonly tx_result?: TxData;
    };
  },
  {
    Method: "commit";
    Parameters: {
      readonly height?: string;
    };
    ReturnType: {
      readonly signed_header: {
        readonly header: Header;
        readonly commit: Commit;
      };
      readonly canonical: boolean;
    };
  },
  {
    Method: "genesis";
    Parameters?: undefined;
    ReturnType: {
      readonly genesis_time: string;
      readonly chain_id: string;
      readonly consensus_params: ConsensusParams;
      // The validators key is used to specify a set of validators for testnets or PoA blockchains.
      // PoS blockchains use the app_state.genutil.gentxs field to stake and bond a number of validators in the first block.
      readonly validators?: readonly ValidatorGenesis[];
      readonly app_hash: Hex;
      readonly app_state: Record<string, unknown> | undefined;
    };
  },
  {
    Method: "health";
    Parameters?: undefined;
    ReturnType: null;
  },
  {
    Method: "num_unconfirmed_txs";
    Parameters?: undefined;
    ReturnType: {
      readonly total: string;
      readonly total_bytes: string;
    };
  },
  {
    Method: "status";
    Parameters?: undefined;
    ReturnType: {
      readonly node_info: NodeInfo;
      readonly sync_info: SyncInfo;
      readonly validator_info: Validator;
    };
  },
  {
    Method: "subscribe";
    Parameters: {
      readonly query: {
        readonly type: SubscriptionEvents;
        readonly raw?: string;
      };
    };
    ReturnType?: undefined;
  },
  {
    Method: "tx";
    Parameters: {
      readonly hash: Base64;
      readonly prove?: boolean;
    };
    ReturnType: TxResponse;
  },
  {
    Method: "tx_search";
    Parameters: {
      readonly query: string;
      readonly prove?: boolean;
      readonly page?: string;
      readonly per_page?: string;
      readonly order_by?: string;
    };
    ReturnType: {
      readonly txs: readonly TxResponse[];
      readonly total_count: string;
    };
  },
  {
    Method: "validators";
    Parameters: {
      readonly height?: string;
      readonly page?: string;
      readonly per_page?: string;
    };
    ReturnType: {
      readonly block_height: string;
      readonly validators: readonly Validator[];
      readonly count: string;
      readonly total: string;
    };
  },
  {
    Method: "unsubscribe";
    Parametes?: undefined;
    ReturnType?: undefined;
  },
];

export type QueryAbciResponse = {
  /**
   * Base64 encoded
   *
   * This can be null since this is a byte slice and due to
   * https://github.com/tendermint/tendermint/blob/v0.35.7/abci/types/result.go#L53
   */
  readonly key?: Base64 | null;
  /**
   * Base64 encoded
   *
   * This can be null since this is a byte slice and due to
   * https://github.com/tendermint/tendermint/blob/v0.35.7/abci/types/result.go#L53
   */
  readonly value?: Base64 | null;
  readonly proofOps?: { ops: ProofOp[] } | null;
  readonly height?: string;
  readonly index?: string;
  readonly code?: number; // only for errors
  readonly codespace?: string;
  readonly log?: string;
  readonly info?: string;
};

export type TxResponse = {
  readonly tx: Base64;
  readonly tx_result: TxData;
  readonly height: string;
  readonly index: number;
  readonly hash: Hex;
  readonly proof?: TxProof;
};

export type TxProof = {
  readonly data: Base64;
  readonly root_hash: Hex;
  readonly proof: {
    readonly total: string;
    readonly index: string;
    readonly leaf_hash: Base64;
    readonly aunts: Base64[];
  };
};

export type TxData = {
  readonly codespace?: string;
  readonly code?: number;
  readonly log?: string;
  readonly data?: Base64;
  readonly events?: TxEvent[];
  readonly gas_wanted?: string;
  readonly gas_used?: string;
};

export type BlockVersion = {
  readonly block: string;
  readonly app?: string;
};

export type Header = {
  readonly version: BlockVersion;
  readonly chain_id: string;
  readonly height: string;
  readonly time: string;
  readonly last_block_id: BlockId;
  readonly last_commit_hash: Hex;
  readonly data_hash: Hex;
  readonly validators_hash: Hex;
  readonly next_validators_hash: Hex;
  readonly consensus_hash: Hex;
  readonly app_hash: Hex;
  readonly last_results_hash: Hex;
  readonly evidence_hash: Hex;
  readonly proposer_address: Hex;
};

/**
 * We lost track on how the evidence structure actually looks like.
 * This is any now and passed to the caller untouched.
 *
 * See also https://github.com/cosmos/cosmjs/issues/980.
 */
export type Evidence = any;

export type Commit = {
  readonly block_id: BlockId;
  readonly height: string;
  readonly round: string;
  readonly signatures: readonly CommitSignature[];
};

export type CommitSignature = {
  readonly block_id_flag: number;
  /** hex encoded */
  readonly validator_address: string;
  readonly timestamp: string;
  /**
   * Base64 encoded signature.
   * There are cases when this is not set, see https://github.com/cosmos/cosmjs/issues/704#issuecomment-797122415.
   */
  readonly signature: Base64 | null;
};

export const BlockIdFlag = {
  Absent: 1,
  Commit: 2,
  Nil: 3,
  Unknown: 0,
  Unrecognized: -1,
} as const;

export type BlockIdFlags = (typeof BlockIdFlag)[keyof typeof BlockIdFlag];

export type BlockMeta = {
  readonly block_id: BlockId;
  readonly block_size: string;
  readonly header: Header;
  readonly num_txs: string;
};

export type Block = {
  readonly header: Header;
  /**
   * For the block at height 1, last commit is not set.
   */
  readonly lastCommit: Commit | null;
  readonly txs: readonly Uint8Array[];
  readonly evidence: readonly Evidence[];
};

export type BlockId = {
  readonly hash: Hex;
  readonly parts: {
    readonly total: number;
    readonly hash: Hex;
  };
};

export type NodeInfo = {
  readonly id: Hex;
  /** IP and port */
  readonly listen_addr: string;
  readonly network: string;
  readonly version: string;
  readonly channels: string; // ???
  readonly moniker: string;
  readonly protocol_version: {
    readonly p2p: string;
    readonly block: string;
    readonly app: string;
  };
  /**
   * Additional information. E.g.
   * {
   *   "tx_index": "on",
   *   "rpc_address":"tcp://0.0.0.0:26657"
   * }
   */
  readonly other: Record<string, unknown>;
};

export type SyncInfo = {
  readonly earliest_app_hash: Hex;
  readonly earliest_block_hash: Hex;
  readonly earliest_block_height: string;
  readonly earliest_block_time: string;
  readonly latest_block_hash: Hex;
  readonly latest_app_hash: Hex;
  readonly latest_block_height: string;
  readonly latest_block_time: string;
  readonly catching_up: boolean;
};

export type Validator = {
  readonly address: Hex;
  readonly pub_key: PubKey;
  readonly voting_power: string;
  readonly proposer_priority?: string;
};

export type ValidatorUpdate = {
  readonly pub_key: PubKey;
  // When omitted, this means zero (see https://github.com/cosmos/cosmjs/issues/1177#issuecomment-1160115080)
  readonly power?: string;
};

export type ConsensusParams = {
  readonly block: BlockParams;
  readonly evidence: EvidenceParams;
};

export type BlockParams = {
  readonly max_bytes: string;
  readonly max_gas: string;
};

export type EvidenceParams = {
  readonly max_age_num_blocks: string;
  readonly max_age_duration: string;
};

export type ProofOp = {
  readonly type: string;
  readonly key: Base64;
  readonly data: Base64;
};

export type PubKey = {
  readonly type: string;
  readonly value: Base64;
};

export type ValidatorGenesis = {
  readonly address: Hex;
  readonly pub_key: PubKey;
  readonly power: string;
  readonly name?: string;
};

export type ValidatorEd25519Pubkey = {
  readonly algorithm: "ed25519";
  readonly data: Uint8Array;
};

export type ValidatorSecp256k1Pubkey = {
  readonly algorithm: "secp256k1";
  readonly data: Uint8Array;
};

export type ValidatorPubkey = ValidatorEd25519Pubkey | ValidatorSecp256k1Pubkey;

export type TxEvent = {
  readonly type: string;
  /** Can be omitted (see https://github.com/cosmos/cosmjs/pull/1198) */
  readonly attributes?: TxEventAttribute[];
};

export type TxEventAttribute = {
  readonly key: string;
  readonly value?: string;
};

export const SubscritionEvent = {
  NewBlock: "NewBlock",
  NewBlockHeader: "NewBlockHeader",
  Tx: "Tx",
} as const;

export type SubscriptionEvents = (typeof SubscritionEvent)[keyof typeof SubscritionEvent];
