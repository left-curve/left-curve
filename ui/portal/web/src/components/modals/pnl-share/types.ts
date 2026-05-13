export type PnlShareProps = {
  pairId: string;
  symbol: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
  equity: string | null;
};
