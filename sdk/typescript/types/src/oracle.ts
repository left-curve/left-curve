export type Price = {
  /** The price of the token in its humanized form. E.g. the price of 1 ATOM,
   * rather than 1 uatom.
   */
  humanizedPrice: string;
  /** The UNIX timestamp of the price (seconds since UNIX epoch). */
  timestamp: number;
};
