import type { Denom } from "./coins.js";
import type { Json } from "./encoding.js";

export type ChainId = string;

/**
 * Represents a blockchain network.
 *
 * @template custom - Custom properties specific to the chain.
 */
export type Chain<custom extends Json | undefined = Json | undefined> = {
  /**
   * Block explorer for the chain.
   */
  blockExplorer: BlockExplorer;

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
  nativeCoin: Denom;

  /**
   * The URLs for the chain.
   */
  urls: ChainUrls;

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

type ChainUrls = {
  rpc?: string;
  webSocket?: string;
  indexer: string;
};

type BlockExplorer = {
  name: string;
  txPage: string;
  contractPage: string;
  accountPage: string;
};
