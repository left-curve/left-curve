import type { Timestamp } from "@left-curve/sdk/types";
import type { Username } from "./account.js";

export type Metadata = {
  /** The username of the account that signed this transaction */
  username: Username;
  /** Identifies the chain this transaction is intended for. */
  chainId: string;
  /** The nonce this transaction was signed with. */
  nonce: number;
  /** The expiration time of this transaction. */
  expiry?: Timestamp;
};
