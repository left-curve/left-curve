const BASE_SYMBOL = "USDC";
const DEFAULT_QUOTE_SYMBOL = "ETH";

type ConvertParams = {
  from?: string | string[];
  to?: string | string[];
};

export type NormalizedConvertParams = {
  from: string;
  to: string;
  changed: boolean;
};

const firstValue = (value?: string | string[]) => {
  if (Array.isArray(value)) return value[0];
  return value;
};

export const normalizeConvertParams = (
  params: ConvertParams,
  coins:
    | {
        bySymbol: Record<string, unknown>;
      }
    | undefined,
): NormalizedConvertParams => {
  const rawFrom = firstValue(params.from);
  const rawTo = firstValue(params.to);
  const requestedFrom = rawFrom || BASE_SYMBOL;
  const requestedTo = rawTo || DEFAULT_QUOTE_SYMBOL;
  const missing = !rawFrom || !rawTo;

  if (!coins) {
    const changed = missing || requestedFrom !== BASE_SYMBOL || requestedTo !== DEFAULT_QUOTE_SYMBOL;
    return { from: BASE_SYMBOL, to: DEFAULT_QUOTE_SYMBOL, changed };
  }

  const fromCoin = coins.bySymbol[requestedFrom];
  const toCoin = coins.bySymbol[requestedTo];

  const isValidPair =
    !!fromCoin && !!toCoin &&
    ((requestedFrom === BASE_SYMBOL && requestedTo !== BASE_SYMBOL) ||
      (requestedTo === BASE_SYMBOL && requestedFrom !== BASE_SYMBOL));

  if (isValidPair) {
    return { from: requestedFrom, to: requestedTo, changed: missing };
  }

  return {
    from: BASE_SYMBOL,
    to: DEFAULT_QUOTE_SYMBOL,
    changed: true,
  };
};
