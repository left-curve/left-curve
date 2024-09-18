import type { NativeCurrency } from "./currency";
import type { Json } from "./encoding";

export type ChainId = string;

/**
 * Represents a blockchain network.
 *
 * @template custom - Custom properties specific to the chain.
 */
export type Chain<custom extends Json | undefined = Json | undefined> = {
  /**
   * Block explorers for the chain.
   */
  blockExplorers?: {
    [key: string]: BlockExplorer;
    default: BlockExplorer;
  };

  /**
   * Contracts for the chain.
   * This is an optional property.
   */
  contracts?: { [key: string]: string } | undefined;

  /**
   * The ID of the chain.
   */
  id: ChainId;

  /**
   * The name of the chain.
   */
  name: string;

  /**
   * The native currency of the chain.
   */
  nativeCurrency: NativeCurrency;

  /**
   * The RPC URLs for the chain.
   */
  rpcUrls: {
    [key: string]: ChainRpcUrls;
    default: ChainRpcUrls;
  };

  /**
   * Indicates if the chain is a testnet.
   * This is an optional property.
   */
  testnet?: boolean | undefined;

  /**
   * Custom properties specific to the chain.
   * This is an optional property.
   */
  custom?: custom | undefined;

  /**
   * The fees for the chain.
   * This is an optional property.
   */
  fees?: ChainFees | undefined;
};

/**
 * Represents the fees for a chain.
 */
export type ChainFees = {
  /**
   * The base fee multiplier.
   * @default 1.4
   */
  baseFeeMultiplier: number;
};

type ChainRpcUrls = {
  http: readonly string[];
  webSocket?: readonly string[] | undefined;
};

type BlockExplorer = {
  name: string;
  txPage: string;
  accountPage: string;
};
