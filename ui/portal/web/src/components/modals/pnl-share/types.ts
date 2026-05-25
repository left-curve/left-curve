/**
 * Two shapes feed the PnL share card:
 *
 * - `position` — an open position. The card derives a live PnL percentage
 *   from entry vs current mark price, and shows leverage based on current
 *   equity.
 * - `fill` — a historical fill (perps trade history row). The realized PnL
 *   is already known in absolute USD; there's no live mark or equity, so
 *   leverage is omitted and the percentage is derived from realized PnL
 *   over the fill's notional.
 */
export type PnlShareProps =
  | {
      mode: "position";
      pairId: string;
      symbol: string;
      size: string;
      entryPrice: string;
      currentPrice: number;
      pnl: number;
      equity: string | null;
    }
  | {
      mode: "fill";
      pairId: string;
      symbol: string;
      size: string;
      fillPrice: string;
      realizedPnl: string;
      createdAt: string;
    };
