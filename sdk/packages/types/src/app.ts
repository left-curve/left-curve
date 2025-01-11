import type { Address } from "./address.js";
import type { Hex } from "./encoding.js";

export type Duration = number;

export type Timestamp = Duration;

/**
 * Only the owner can perform the action. Note, the owner is always able to
 * upload code or instantiate contracts.
 */
export type NobodyPermission = "nobody";
/**
 * Any account is allowed to perform the action
 */
export type EverybodyPermission = "everybody";
/**
 * Some whitelisted accounts or the owner can perform the action.
 */
export type SomebodiesPermission = { somebodies: Address[] };

/**
 * Permissions for uploading code or instantiating contracts.
 */
export type Permission = NobodyPermission | EverybodyPermission | SomebodiesPermission;

export type BlockInfo = {
  height: string;
  timestamp: string;
  hash: string;
};

export type ContractInfo = {
  codeHash: Hex;
  label?: string;
  admin?: Address;
};

export type ChainConfig = {
  /** The account that can update this config. */
  owner: Address;
  /** The contract the manages fungible token transfers. */
  bank: Address;
  /** The contract that handles transaction fees. */
  taxman: Address;
  /** A list of contracts that are to be called at regular time intervals. */
  cronjobs: Record<Address, Duration>;
  /** Permissions for certain gated actions. */
  permissions: {
    upload: Permission;
    instantiate: Permission;
  };
  /** Maximum age allowed for orphaned codes.
   * A code is deleted if it remains orphaned (not used by any contract) for
   * longer than this duration.
   */
  maxOrphanAge: Duration;
};
