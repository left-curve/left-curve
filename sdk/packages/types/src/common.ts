import type { Address } from "./address";

export type Duration = number;

export type Timestamp = Duration;

export type Language = `${string}-${string}`;

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
