import { createContext } from "@left-curve/applets-kit";
import { type AttackCompensation, useAccount, usePoints } from "@left-curve/store";
import type React from "react";
import { type PropsWithChildren, useMemo } from "react";

export type UserLeague =
  | "wood"
  | "iron"
  | "gold"
  | "platinum"
  | "diamond"
  | "master"
  | "grandmaster";

type LeagueConfig = {
  key: UserLeague;
  label: string;
  minPercentile: number;
  maxPercentile: number;
};

const LEAGUE_CONFIG: LeagueConfig[] = [
  { key: "wood", label: "Wood", minPercentile: 0, maxPercentile: 30 },
  { key: "iron", label: "Iron", minPercentile: 30, maxPercentile: 55 },
  { key: "gold", label: "Gold", minPercentile: 55, maxPercentile: 73 },
  { key: "platinum", label: "Platinum", minPercentile: 73, maxPercentile: 85 },
  { key: "diamond", label: "Diamond", minPercentile: 85, maxPercentile: 93 },
  { key: "master", label: "Master", minPercentile: 93, maxPercentile: 98 },
  { key: "grandmaster", label: "Grandmaster", minPercentile: 98, maxPercentile: 100 },
];

const getLeagueFromPercentile = (percentile: number): LeagueConfig => {
  const clampedPercentile = Math.min(Math.max(percentile, 0), 100);

  for (const league of LEAGUE_CONFIG) {
    if (clampedPercentile >= league.minPercentile && clampedPercentile < league.maxPercentile) {
      return league;
    }
  }

  return LEAGUE_CONFIG[LEAGUE_CONFIG.length - 1];
};

const getNextLeague = (currentLeague: UserLeague): LeagueConfig | null => {
  const currentIndex = LEAGUE_CONFIG.findIndex((l) => l.key === currentLeague);
  if (currentIndex === -1 || currentIndex === LEAGUE_CONFIG.length - 1) {
    return null;
  }
  return LEAGUE_CONFIG[currentIndex + 1];
};

type UserPointsData = {
  points: number;
  volume: number;
  pnl: number;
  rank: number;
  percentile: number;
  league: UserLeague;
  leagueConfig: LeagueConfig;
  nextLeague: LeagueConfig | null;
  tradingPoints: number;
  lpPoints: number;
  referralPoints: number;
  compensation: AttackCompensation | undefined;
};

type UserPointsContextValue = UserPointsData & {
  isLoading: boolean;
  leagueList: LeagueConfig[];
};

const [UserPointsContextProvider, useUserPointsContext] = createContext<UserPointsContextValue>({
  name: "UserPointsContext",
});

export const UserPointsProvider: React.FC<PropsWithChildren> = ({ children }) => {
  const { userIndex } = useAccount();
  const pointsUrl = window.dango.urls.pointsUrl;

  const {
    points,
    lpPoints,
    tradingPoints,
    referralPoints,
    volume,
    pnl,
    rank,
    percentile,
    compensation,
    isLoading,
  } = usePoints({ pointsUrl, userIndex });

  const value = useMemo(() => {
    const leagueConfig = getLeagueFromPercentile(percentile);
    const league = leagueConfig.key;
    const nextLeague = getNextLeague(league);

    return {
      points,
      volume,
      pnl,
      rank,
      percentile,
      league,
      leagueConfig,
      nextLeague,
      tradingPoints,
      lpPoints,
      referralPoints,
      compensation,
      isLoading,
      leagueList: LEAGUE_CONFIG,
    };
  }, [points, lpPoints, tradingPoints, referralPoints, volume, pnl, rank, percentile, compensation, isLoading]);

  return <UserPointsContextProvider value={value}>{children}</UserPointsContextProvider>;
};

export const useUserPoints = useUserPointsContext;

export { LEAGUE_CONFIG, getLeagueFromPercentile, getNextLeague };
export type { LeagueConfig };
