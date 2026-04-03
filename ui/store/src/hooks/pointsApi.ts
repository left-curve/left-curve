export type Points = {
  vault: string;
  perps: string;
  referral: string;
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
};

export type BoxReward = {
  box_id: string;
  chest: "Bronze" | "Silver" | "Gold" | "Crystal";
  loot: "Common" | "Uncommon" | "Rare" | "Epic" | "Legendary" | "Mythic";
  opened: boolean;
};

export type OatEntry = {
  collection_id: number;
  token_id: string;
  /** Seconds with nanosecond decimal precision, serialized as a string by the backend (e.g. "1743460800.000000000") */
  registered_at: string;
};

export const fetchUserStats = async (baseUrl: string, userIndex: number): Promise<UserPoints> => {
  const res = await fetch(`${baseUrl}/stats/user/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch user stats: ${res.status}`);
  return res.json();
};

export const fetchEpochPoints = async (
  baseUrl: string,
  userIndex: number,
  params?: { min?: number; max?: number },
): Promise<Record<string, EpochUserStats>> => {
  const url = new URL(`${baseUrl}/stats/user/${userIndex}/epochs`);
  if (params?.min !== undefined) url.searchParams.set("min", String(params.min));
  if (params?.max !== undefined) url.searchParams.set("max", String(params.max));
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

export const fetchUserBoxes = async (baseUrl: string, userIndex: number): Promise<BoxReward[]> => {
  const res = await fetch(`${baseUrl}/boxes/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch boxes: ${res.status}`);
  return res.json();
};

export const openBox = async (
  baseUrl: string,
  userIndex: number,
  boxId: string,
): Promise<{ success: boolean }> => {
  const res = await fetch(`${baseUrl}/boxes/open`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ user_index: userIndex, box_id: boxId }),
  });
  if (!res.ok) throw new Error(`Failed to open box: ${res.status}`);
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
  if (!res.ok) throw new Error(`Failed to register OAT: ${res.status}`);
  return res.json();
};

export type OatCheckEntry = {
  collection_id: number;
  token_id: string;
  maybe_username: { index: number } | null;
};

export const checkOat = async (
  baseUrl: string,
  evmAddress: string,
): Promise<OatCheckEntry[]> => {
  const res = await fetch(`${baseUrl}/oat/check/${evmAddress}`);
  if (!res.ok) throw new Error(`Failed to check OAT: ${res.status}`);
  return res.json();
};

export type EpochInfoNotStarted = {
  status: "not_started";
  starts_at: { Block: number } | { Timestamp: string };
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
