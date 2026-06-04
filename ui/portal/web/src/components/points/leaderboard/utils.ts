import { m } from "@left-curve/foundation/paraglide/messages.js";

export function formatUsername(username: string | null, userIndex: number): string {
  if (!username) return m["points.leaderboard.userFallback"]({ index: String(userIndex) });
  const match = username.match(/^user_(\d+)$/);
  if (match) return m["points.leaderboard.userFallback"]({ index: match[1] });
  return username;
}
