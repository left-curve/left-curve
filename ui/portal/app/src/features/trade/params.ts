type TradeParams = {
  pairSymbols?: string | string[];
  action?: string | string[];
  order_type?: string | string[];
};

type CoinMap = {
  bySymbol: Record<string, { denom: string } | undefined>;
};

type PairValidation = {
  baseSymbol: string;
  quoteSymbol: string;
  baseDenom: string;
  quoteDenom: string;
};

export type NormalizedTradeParams = {
  pairSymbols: string;
  action: "buy" | "sell";
  orderType: "limit" | "market";
  changed: boolean;
  pair: PairValidation;
};

const DEFAULT_PAIR = "ETH-USDC";
const DEFAULT_ACTION: NormalizedTradeParams["action"] = "buy";
const DEFAULT_ORDER_TYPE: NormalizedTradeParams["orderType"] = "market";

const firstValue = (value?: string | string[]) => (Array.isArray(value) ? value[0] : value);

export const normalizeTradeParams = (
  params: TradeParams,
  coins: CoinMap | undefined,
): NormalizedTradeParams => {
  const rawPair = firstValue(params.pairSymbols) || DEFAULT_PAIR;
  const rawAction = firstValue(params.action);
  const rawOrderType = firstValue(params.order_type);

  const action: NormalizedTradeParams["action"] = rawAction === "sell" ? "sell" : "buy";
  const orderType: NormalizedTradeParams["orderType"] =
    rawOrderType === "limit" ? "limit" : "market";

  const [baseSymbol, quoteSymbol] = rawPair.split("-");
  const hasPairFormat = !!baseSymbol && !!quoteSymbol && rawPair.split("-").length === 2;

  const fallback = {
    baseSymbol: "ETH",
    quoteSymbol: "USDC",
    baseDenom: "",
    quoteDenom: "",
  };

  if (!coins) {
    return {
      pairSymbols: `${fallback.baseSymbol}-${fallback.quoteSymbol}`,
      action,
      orderType,
      changed: !hasPairFormat || rawPair !== DEFAULT_PAIR || action !== rawAction || orderType !== rawOrderType,
      pair: fallback,
    };
  }

  const baseCoin = hasPairFormat ? coins.bySymbol[baseSymbol] : undefined;
  const quoteCoin = hasPairFormat ? coins.bySymbol[quoteSymbol] : undefined;
  const validPair = !!baseCoin && !!quoteCoin;

  if (validPair && baseCoin && quoteCoin) {
    const normalizedPair = `${baseSymbol}-${quoteSymbol}`;
    return {
      pairSymbols: normalizedPair,
      action,
      orderType,
      changed:
        normalizedPair !== rawPair || action !== rawAction || orderType !== rawOrderType,
      pair: {
        baseSymbol,
        quoteSymbol,
        baseDenom: baseCoin.denom,
        quoteDenom: quoteCoin.denom,
      },
    };
  }

  const defaultBase = coins.bySymbol.ETH;
  const defaultQuote = coins.bySymbol.USDC;

  return {
    pairSymbols: DEFAULT_PAIR,
    action,
    orderType,
    changed: true,
    pair: {
      baseSymbol: "ETH",
      quoteSymbol: "USDC",
      baseDenom: defaultBase?.denom || "",
      quoteDenom: defaultQuote?.denom || "",
    },
  };
};
