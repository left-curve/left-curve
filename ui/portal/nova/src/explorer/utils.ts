import type { IndexedTransaction } from "@left-curve/dango/types";

export function formatTimeAgo(dateStr: string): string {
  const diffMs = Date.now() - new Date(dateStr).getTime();
  const diffSec = Math.floor(diffMs / 1000);

  if (diffSec < 60) return `${diffSec}s ago`;

  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m ago`;

  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;

  const diffDays = Math.floor(diffHr / 24);
  return `${diffDays}d ago`;
}

export function truncateHash(hash: string, startLen = 8, endLen = 6): string {
  if (hash.length <= startLen + endLen + 3) return hash;
  if (endLen === 0) return `${hash.slice(0, startLen)}...`;
  return `${hash.slice(0, startLen)}...${hash.slice(-endLen)}`;
}

export function primaryMethodName(tx: IndexedTransaction): string {
  if (tx.messages.length === 0) return tx.transactionType;
  return tx.messages[0].methodName;
}
