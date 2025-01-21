import type { Address, Coin, Denom } from "@left-curve/types";
import type { Username } from "./account.js";

export type TokenFactoryConfig = {
  /** The amount of fee that must be paid in order to create a denom. */
  readonly tokenCreationFee?: Coin;
};

export type TokenFactoryQueryMsg =
  /** Query the TokenFactory's global configuration. */
  | { config: Record<never, never> }
  /** Query a denom's admin address. */
  | { admin: { denom: Denom } }
  /** Enumerate all denoms and their admin addresses. */
  | { admins: { startAfter?: Denom; limit?: number } };

export type TokenFactoryExecuteMsg =
  /** Update the configurations. It can only be called by the chain owner.  */
  | { configure: { new_cfg: TokenFactoryConfig } }
  /** Mint the token of the specified subdenom and amount to a recipient. */
  | { mint: { denom: Denom; to: Address; amount: string } }
  /** Burn the token of the specified subdenom and amount from a source. */
  | { burn: { denom: Denom; from: Address; amount: string } }
  /** Create a new token with the given sub-denomination, and appoint an admin
   * who can mint or burn this token.
   * The creator must attach exactly the amount of denom creation fee
   * along with the call.
   */
  | {
      create: {
        subdenom: Denom;
        /** If provided, the denom will be formatted as:
         * > factory/{username}/{subdenom}
         * Otherwise, it will be formatted as:
         * > factory/{sender_address}/{subdenom}
         */
        username?: Username;
        /** If not provided, use the message sender's address. */
        admin?: Address;
      };
    };
