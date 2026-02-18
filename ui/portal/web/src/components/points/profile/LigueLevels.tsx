import { twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";
import { useUserPoints, type UserLeague } from "../useUserPoints";

type LigueLevelKey = UserLeague;

type LigueLevel = {
  key: LigueLevelKey;
  image: string;
  pillarClasses: string;
};

type LevelState = "past" | "current" | "future";

const LIGUE_LEVELS: LigueLevel[] = [
  {
    key: "wood",
    image: "/images/points/ligue/wood.png",
    pillarClasses: "h-[42px] lg:h-[104px]",
  },
  {
    key: "iron",
    image: "/images/points/ligue/iron.png",
    pillarClasses: "h-[58px] lg:h-[145px]",
  },
  {
    key: "gold",
    image: "/images/points/ligue/gold.png",
    pillarClasses: "h-[66px] lg:h-[166px]",
  },
  {
    key: "platinum",
    image: "/images/points/ligue/platinum.png",
    pillarClasses: "h-[72px] lg:h-[181px]",
  },
  {
    key: "diamond",
    image: "/images/points/ligue/diamond.png",
    pillarClasses: "h-[85px] lg:h-[212px]",
  },
  {
    key: "master",
    image: "/images/points/ligue/master.png",
    pillarClasses: "h-[98px] lg:h-[245px]",
  },
  {
    key: "grandmaster",
    image: "/images/points/ligue/grandmaster.png",
    pillarClasses: "h-[105px] lg:h-[263px]",
  },
];

const getLevelState = (levelIndex: number, currentLevelIndex: number): LevelState => {
  if (levelIndex < currentLevelIndex) return "past";
  if (levelIndex === currentLevelIndex) return "current";
  return "future";
};

type LigueLevelItemProps = {
  level: LigueLevel;
  state: LevelState;
};

const LigueLevelItem: React.FC<LigueLevelItemProps> = ({ level, state }) => {
  const isFuture = state === "future";
  const isCurrent = state === "current";

  return (
    <div className="relative flex flex-col items-center justify-end flex-1 min-w-0 overflow-visible">
      {isCurrent && (
        <img
          src="/images/points/league-shine.png"
          alt=""
          className="absolute bottom-full left-1/2 -translate-x-[48%] translate-y-1/2 lg:translate-y-[45%] w-[120px] h-[120px] lg:w-[260px] lg:h-[260px] object-contain z-0 drag-none select-none max-w-none"
        />
      )}
      <img
        src={level.image}
        alt={`${level.key} badge`}
        className={twMerge(
          "absolute bottom-full left-1/2 -translate-x-1/2 translate-y-[60%] lg:translate-y-1/2 w-[80px] h-[80px] lg:w-[200px] lg:h-[200px] object-contain z-10 drag-none select-none max-w-none",
          isFuture && "grayscale",
        )}
      />
      <div
        className={twMerge(
          "w-full  flex flex-col items-center justify-end rounded-t-sm bg-gradient-to-b from-surface-quaternary-rice-hover to-surface-primary-rice",
          level.pillarClasses,
          isFuture && "from-surface-tertiary-gray",
        )}
      >
        {isCurrent && (
          <>
            <div className="absolute left-[0.5rem] top-0 w-[2px] h-full bg-gradient-to-b from-fg-secondary-rice to-surface-primary-rice" />
            <div className="absolute right-[0.5rem] top-0 w-[2px] h-full bg-gradient-to-b from-fg-secondary-rice to-surface-primary-rice" />
          </>
        )}
        <p
          className={twMerge(
            "exposure-xs-italic lg:exposure-sm-italic text-ink-secondary-rice pb-2 lg:pb-3 text-center whitespace-nowrap",
            isFuture && "text-gray-400",
            !isCurrent && "hidden md:block",
          )}
        >
          {m["points.leagues.levels"]({ level: level.key })}
        </p>
      </div>
    </div>
  );
};

type LigueLevelsProps = {
  currentLevel?: LigueLevelKey;
};

export const LigueLevels: React.FC<LigueLevelsProps> = ({ currentLevel }) => {
  const { league } = useUserPoints();
  const level = currentLevel ?? league;
  const currentLevelIndex = LIGUE_LEVELS.findIndex((l) => l.key === level);

  return (
    <div className="w-full min-h-[18rem] lg:min-h-[28rem] bg-surface-primary-rice rounded-b-xl p-4 lg:p-8 flex flex-col">
      <div className="flex flex-col mb-4 lg:mb-6">
        <h2 className="display-heading-4xs lg:display-heading-xl text-primitives-warning-300">
          {m["points.leagues.title"]()}
        </h2>
        <p>{m["points.leagues.subtitle"]()}</p>
      </div>
      <div className="flex-1 flex items-end justify-between gap-2 md:gap-4 lg:gap-8 pt-[70px] lg:pt-[110px]">
        {LIGUE_LEVELS.map((level, index) => (
          <LigueLevelItem
            key={level.key}
            level={level}
            state={getLevelState(index, currentLevelIndex)}
          />
        ))}
      </div>
    </div>
  );
};
