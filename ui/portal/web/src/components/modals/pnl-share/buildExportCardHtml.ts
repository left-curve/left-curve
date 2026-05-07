import { formatPrice } from "./utils.js";

type TrustedValues = {
  symbol: string;
  entryPrice: string;
  currentPrice: number;
  displayPercent: number;
  isPositive: boolean;
  referralLink: string | undefined;
};

export function cloneCardForExport(
  source: HTMLElement,
  values: TrustedValues,
): HTMLElement {
  const clone = source.cloneNode(true) as HTMLElement;

  const pctText = `${values.isPositive ? "+" : ""}${values.displayPercent.toFixed(2)}%`;

  const overrides: Record<string, string> = {
    symbol: values.symbol,
    percent: pctText,
    "entry-price": formatPrice(Number(values.entryPrice)),
    "mark-price": formatPrice(values.currentPrice),
  };

  if (values.referralLink) {
    overrides.referral = values.referralLink;
  }

  for (const [key, text] of Object.entries(overrides)) {
    const el = clone.querySelector(`[data-pnl="${key}"]`);
    if (el) el.textContent = text;
  }

  // Force desktop sizes for the character image (overrides responsive Tailwind classes)
  const characterImg = clone.querySelector("img[alt='character']") as HTMLElement | null;
  if (characterImg) {
    characterImg.style.height = "80%";
    characterImg.style.maxHeight = "17rem";
  }

  // Force desktop layout for the prices row
  const pricesRow = clone.querySelector("[data-pnl='entry-price']")?.closest("div.flex.flex-col") ?.parentElement as HTMLElement | null;
  if (pricesRow) {
    pricesRow.style.flexDirection = "row";
    pricesRow.style.gap = "1.5rem";
  }

  return clone;
}
