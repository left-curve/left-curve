export type Price = {
  /** The price of the token in its humanized form. E.g. the price of 1 ATOM,
   * rather than 1 uatom.
   */
  humanizedPrice: string;
  /** The UNIX timestamp of the price (seconds since UNIX epoch). */
  timestamp: number;
  /** The market session at which the price was observed. For 24/7 markets
   * (e.g. crypto) this is always `"regular"`. `"other"` covers any
   * non-regular state (pre/post-market, overnight, closed) as well as
   * payloads that omit the property.
   */
  marketSession: "regular" | "other";
};
