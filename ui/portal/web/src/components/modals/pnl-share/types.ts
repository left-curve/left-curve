export type PnlShareProps = {
  pairId: string;
  symbol: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
  pnl: number;
};

export type PnlCardData = {
  symbol: string;
  entryPrice: string;
  currentPrice: number;
  displayPercent: number;
  isPositive: boolean;
  isLong: boolean;
  characterImg: string;
  dangoLogoSrc: string;
  logoURI: string | undefined;
  referralLink: string | undefined;
};
