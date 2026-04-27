export type Price = {
  /* The price of the token in its humanized form. I.e. the price of 1 ATOM,
    rather than 1 uatom.
  */
  humanizedPrice: string;
  /* The exponential moving average of the price of the token in its
    humanized form.
  */
  humanizedEma: string;
  /* The UNIX timestamp of the price (seconds since UNIX epoch).
   */
  precision: number;
  /* The number of decimal places of the token that is used to convert
    the price from its smallest unit to a humanized form. E.g. 1 ATOM
    is 10^6 uatom, so the precision is 6.
  */
  timestamp: number;
};
