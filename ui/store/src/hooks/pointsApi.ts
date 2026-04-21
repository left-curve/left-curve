export type Points = {
  vault: string;
  perps: string;
  referral: string;
};

export type AttackCompensation = {
  vault: string;
  unrealized: string;
};

export type UserStats = {
  points: Points;
  realized_pnl: string;
  volume: string;
};

export type EpochUserStats = {
  stats: UserStats;
  started_at: string;
  ended_at: string;
};

export type LeaderboardEntry = {
  user_index: number;
  username: string | null;
  stats: UserStats;
};

export type UserPoints = {
  stats: UserStats;
  rank: number;
  compensation?: AttackCompensation;
};

export type BoxCount = {
  total: number;
  opened: number;
};

export type BoxesResponse = Record<string, Record<string, BoxCount>>;

export type OatEntry = {
  collection_id: number;
  token_id: string;
  /** Seconds with nanosecond decimal precision, serialized as a string by the backend (e.g. "1743460800.000000000") */
  registered_at: string;
};

export class OatRateLimitError extends Error {
  retryAfterSeconds: number;

  constructor(retryAfterSeconds: number) {
    super(`Address already linked recently. Retry in ${retryAfterSeconds} seconds.`);
    this.name = "OatRateLimitError";
    this.retryAfterSeconds = retryAfterSeconds;
  }
}

export class NoOatsFoundError extends Error {
  constructor() {
    super("No OATs found for this address");
    this.name = "NoOatsFoundError";
  }
}

function parseRetrySeconds(message: string): number {
  const match = message.match(/retry in (\d+) second/i);
  return match ? Number.parseInt(match[1], 10) : 60;
}

function isNoOatsFoundError(text: string): boolean {
  return text.includes("empty_address_or_galxe_id");
}

export const fetchUserStats = async (baseUrl: string, userIndex: number): Promise<UserPoints> => {
  const res = await fetch(`${baseUrl}/stats/user/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch user stats: ${res.status}`);
  return res.json();
};

export const fetchEpochPoints = async (
  baseUrl: string,
  userIndex: number,
  params?: { min?: number; max?: number; order?: "asc" | "desc" },
): Promise<[number, EpochUserStats][]> => {
  const url = new URL(`${baseUrl}/stats/user/${userIndex}/epochs`);
  if (params?.min !== undefined) url.searchParams.set("min", String(params.min));
  if (params?.max !== undefined) url.searchParams.set("max", String(params.max));
  if (params?.order) url.searchParams.set("order", params.order);
  const res = await fetch(url.toString());
  if (!res.ok) throw new Error(`Failed to fetch epoch points: ${res.status}`);
  return res.json();
};

export const fetchLeaderboard = async (
  baseUrl: string,
  params?: { sort?: string; timeframe?: number },
): Promise<LeaderboardEntry[]> => {
  const url = new URL(`${baseUrl}/leaderboard`);
  if (params?.sort) url.searchParams.set("sort", params.sort);
  if (params?.timeframe !== undefined) url.searchParams.set("timeframe", String(params.timeframe));
  const res = await fetch(url.toString());
  if (!res.ok) throw new Error(`Failed to fetch leaderboard: ${res.status}`);
  return res.json();
};

export const fetchTotalUsersWithPoints = async (baseUrl: string): Promise<number> => {
  const res = await fetch(`${baseUrl}/stats/total-users-with-points`);
  if (!res.ok) throw new Error(`Failed to fetch total users: ${res.status}`);
  return res.json();
};

export const fetchUserBoxes = async (
  baseUrl: string,
  userIndex: number,
): Promise<BoxesResponse> => {
  const res = await fetch(`${baseUrl}/boxes/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch boxes: ${res.status}`);
  return res.json();
};

export const openBoxes = async (
  baseUrl: string,
  userIndex: number,
  boxes: Record<string, Record<string, number>>,
): Promise<{ success: boolean }> => {
  const res = await fetch(`${baseUrl}/boxes/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ user_index: userIndex, boxes }),
  });
  if (!res.ok) throw new Error(`Failed to open boxes: ${res.status}`);
  return res.json();
};

export const fetchUserOats = async (baseUrl: string, userIndex: number): Promise<OatEntry[]> => {
  const res = await fetch(`${baseUrl}/oat/user/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch OATs: ${res.status}`);
  return res.json();
};

export const fetchCampaigns = async (baseUrl: string): Promise<[string, number][]> => {
  const res = await fetch(`${baseUrl}/oat/campaigns`);
  if (!res.ok) throw new Error(`Failed to fetch campaigns: ${res.status}`);
  return res.json();
};

export const registerOat = async (
  baseUrl: string,
  body: {
    user_index: number;
    evm_address: string;
    signature: unknown;
  },
): Promise<unknown> => {
  const res = await fetch(`${baseUrl}/oat/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const text = await res.text();

    if (res.status === 429) {
      throw new OatRateLimitError(parseRetrySeconds(text));
    }

    if (isNoOatsFoundError(text)) {
      throw new NoOatsFoundError();
    }

    throw new Error(`Failed to register OAT: ${res.status}`);
  }

  return res.json();
};

export const checkOat = async (
  baseUrl: string,
  evmAddress: string,
): Promise<Record<number, string>> => {
  const res = await fetch(`${baseUrl}/oat/check/${evmAddress}`);
  if (!res.ok) throw new Error(`Failed to check OAT: ${res.status}`);
  return res.json();
};

export type PredictPointsEvent = {
  epoch: number;
  stats: UserStats | null;
  updated_at_epoch_secs: number;
  next_update_epoch_secs: number;
};

export type EpochInfoNotStarted = {
  status: "not_started";
  starts_at: { block: number } | { timestamp: string };
};

export type EpochInfoActive = {
  status: "active";
  current_epoch: number;
  remaining: string;
};

export type EpochInfo = EpochInfoNotStarted | EpochInfoActive;

export const fetchCurrentEpoch = async (baseUrl: string): Promise<EpochInfo> => {
  const res = await fetch(`${baseUrl}/event/epoch`);
  if (!res.ok) throw new Error(`Failed to fetch current epoch: ${res.status}`);
  return res.json();
};
