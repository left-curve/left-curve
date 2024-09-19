export type CurrencyFormatterOptions = {
  currency: string;
  language: string;
};

export function formatCurrency(
  amount: number | bigint,
  { currency, language }: CurrencyFormatterOptions,
) {
  return new Intl.NumberFormat(language, {
    style: "currency",
    currency,
  }).format(amount);
}

export function formatAddress(address: string, sub = 4): string {
  return address.slice(0, 6).concat("...") + address.substring(address.length - sub);
}
