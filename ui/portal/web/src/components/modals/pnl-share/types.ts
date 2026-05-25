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
