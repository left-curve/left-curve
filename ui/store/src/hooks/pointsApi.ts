export type PointsResponse = {
  vault: string;
  trades: string;
  perps: string;
  referral: string;
};

export type LeaderboardEntry = {
  user_index: number;
  points: PointsResponse;
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
  registered_at: number;
};

export const fetchUserPoints = async (
  baseUrl: string,
  userIndex: number,
): Promise<PointsResponse> => {
  const res = await fetch(`${baseUrl}/points/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch points: ${res.status}`);
  return res.json();
};

export const fetchLeaderboard = async (
  baseUrl: string,
  startAfter?: number,
): Promise<Record<string, LeaderboardEntry>> => {
  const url = startAfter
    ? `${baseUrl}/leaderboard/${startAfter}`
    : `${baseUrl}/leaderboard`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch leaderboard: ${res.status}`);
  return res.json();
};

export const fetchUserBoxes = async (
  baseUrl: string,
  userIndex: number,
): Promise<BoxReward[]> => {
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

export const fetchUserOats = async (
  baseUrl: string,
  userIndex: number,
): Promise<OatEntry[]> => {
  const res = await fetch(`${baseUrl}/oats/${userIndex}`);
  if (!res.ok) throw new Error(`Failed to fetch OATs: ${res.status}`);
  return res.json();
};

export const fetchCampaigns = async (baseUrl: string): Promise<[string, number][]> => {
  const res = await fetch(`${baseUrl}/campaigns`);
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
  const res = await fetch(`${baseUrl}/register-oat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Failed to register OAT: ${res.status}`);
  return res.json();
};

export const checkOat = async (
  baseUrl: string,
  evmAddress: string,
): Promise<Record<number, string>> => {
  const res = await fetch(`${baseUrl}/check-oat/${evmAddress}`);
  if (!res.ok) throw new Error(`Failed to check OAT: ${res.status}`);
  return res.json();
};
