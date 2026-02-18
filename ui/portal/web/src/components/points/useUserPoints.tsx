import { createContext } from "@left-curve/applets-kit";
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

/**
 * User leagues based on percentile of points holders:
 * - Wood: bottom 30%
 * - Iron: next 25% (30-55%)
 * - Gold: next 18% (55-73%)
 * - Platinum: next 12% (73-85%)
 * - Diamond: next 8% (85-93%)
 * - Master: next 5% (93-98%)
 * - Grandmaster: top 2% (98-100%)
 */
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
  rank: number;
  percentile: number;
  league: UserLeague;
  leagueConfig: LeagueConfig;
  nextLeague: LeagueConfig | null;
  tradingPoints: number;
  lpPoints: number;
  referralPoints: number;
};

type UserPointsContextValue = UserPointsData & {
  isLoading: boolean;
  leagueList: LeagueConfig[];
};

const [UserPointsContextProvider, useUserPointsContext] = createContext<UserPointsContextValue>({
  name: "UserPointsContext",
});

type UserPointsProviderProps = PropsWithChildren<{
  initialData?: Partial<UserPointsData>;
}>;

export const UserPointsProvider: React.FC<UserPointsProviderProps> = ({
  children,
  initialData,
}) => {
  const value = useMemo(() => {
    // Mock data - will be replaced with actual API data
    const points = initialData?.points ?? 16300;
    const volume = initialData?.volume ?? 75000;
    const rank = initialData?.rank ?? 11200;
    const percentile = initialData?.percentile ?? 94; // Mock: user is in Master league
    const tradingPoints = initialData?.tradingPoints ?? 3000;
    const lpPoints = initialData?.lpPoints ?? 12000;
    const referralPoints = initialData?.referralPoints ?? 8500;

    const leagueConfig = getLeagueFromPercentile(percentile);
    const league = leagueConfig.key;
    const nextLeague = getNextLeague(league);

    return {
      points,
      volume,
      rank,
      percentile,
      league,
      leagueConfig,
      nextLeague,
      tradingPoints,
      lpPoints,
      referralPoints,
      isLoading: false,
      leagueList: LEAGUE_CONFIG,
    };
  }, [initialData]);

  return <UserPointsContextProvider value={value}>{children}</UserPointsContextProvider>;
};

export const useUserPoints = useUserPointsContext;

export { LEAGUE_CONFIG, getLeagueFromPercentile, getNextLeague };
export type { LeagueConfig };
