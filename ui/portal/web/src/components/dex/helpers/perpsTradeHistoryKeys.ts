export const perpsTradeHistoryKeys = {
  all: ["perpsTradeHistory"],
  account: (address: string | undefined) => [...perpsTradeHistoryKeys.all, address ?? ""],
  range: (
    address: string | undefined,
    earlierThan: string | undefined,
    laterThan: string | undefined,
  ) => [...perpsTradeHistoryKeys.account(address), earlierThan ?? "", laterThan ?? ""],
  fillMarkers: (address: string, pairId: string, from: number, to: number) => [
    ...perpsTradeHistoryKeys.account(address),
    "fillMarkers",
    pairId,
    from,
    to,
  ],
};

export function isPerpsTradeHistoryAccountKey(
  queryKey: readonly unknown[],
  address: string,
): boolean {
  const accountKey = perpsTradeHistoryKeys.account(address);
  return accountKey.every((part, index) => queryKey[index] === part);
}
