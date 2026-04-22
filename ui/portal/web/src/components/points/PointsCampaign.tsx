import {
  Button,
  IconDangoStick,
  IconGift,
  IconStar,
  IconUser,
  Tab,
  Tabs,
  createContext,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

import type { PropsWithChildren } from "react";
import { MobileTitle } from "../foundation/MobileTitle";
import { LeaderboardTable } from "./leaderboard";
import { PointsHeader } from "./PointsHeader";
import { LigueLevels, PointsProfileTable } from "./profile";
import {
  BoxesSection,
  ChestOpeningProvider,
  NFTsSection,
  OATsSection,
  PointsProgressBar,
} from "./rewards";
import { UserPointsProvider, useUserPoints } from "./useUserPoints";
import { useAccount, useBoxes, useOats } from "@left-curve/store";

type PointsCampaignTab = "profile" | "rewards" | "leaderboard";

const [PointsCampaignProvider, usePointsCampaign] = createContext<{
  activeTab: PointsCampaignTab;
  setActiveTab: (tab: PointsCampaignTab) => void;
}>({
  name: "PointsCampaignContext",
});

type PointsCampaignContainerProps = PropsWithChildren<{
  activeTab: PointsCampaignTab;
  onTabChange: (tab: PointsCampaignTab) => void;
}>;

const PointsCampaignContainer: React.FC<PointsCampaignContainerProps> = ({
  children,
  activeTab,
  onTabChange,
}) => {
  const { userIndex } = useAccount();

  return (
    <UserPointsProvider>
      <ChestOpeningProviderWrapper userIndex={userIndex}>
        <PointsCampaignProvider value={{ activeTab, setActiveTab: onTabChange }}>
          <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 lg:p-0 lg:pt-6 lg:pb-20 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
            <MobileTitle title={m["points.mobileTitle"]()} />
            <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
              {children}
            </div>
          </div>
        </PointsCampaignProvider>
      </ChestOpeningProviderWrapper>
    </UserPointsProvider>
  );
};

const ChestOpeningProviderWrapper: React.FC<PropsWithChildren<{ userIndex?: number }>> = ({
  children,
  userIndex,
}) => {
  const pointsUrl = window.dango.urls.pointsUrl;
  const { unopenedBoxes } = useBoxes({ pointsUrl, userIndex });

  return (
    <ChestOpeningProvider userIndex={userIndex} unopenedBoxes={unopenedBoxes}>
      {children}
    </ChestOpeningProvider>
  );
};

const PointsCampaignHeader: React.FC = () => (
  <div className="flex flex-col gap-8 w-full items-center">
    <div className="max-w-[15.5rem] flex flex-col gap-2 items-center text-center">
      <p className="text-ink-tertiary-500 diatype-m-regular">{m["points.header.welcome"]()}</p>
      <h1 className="exposure-h1-italic lg:text-[48px] text-ink-primary-rice flex flex-col items-center">
        {(() => {
          const parts = m["points.header.titlePoints"]({ icon: "{icon}" }).split("{icon}");
          return (
            <span className="flex items-center">
              {parts[0]}
              <IconDangoStick className="inline-block h-[2.7rem] w-auto -mx-[0.6rem]" />
              {parts[1]}
            </span>
          );
        })()}
        <span>{m["points.header.titleProgram"]()}</span>
      </h1>
    </div>
    <Button
      variant="utility"
      onClick={() => window.open("https://dango-4.gitbook.io/dango-docs/points")}
    >
      {m["points.header.readRules"]()}
    </Button>
  </div>
);

const ProfileHeader: React.FC = () => {
  return (
    <div className="w-full rounded-xl shadow-account-card overflow-hidden bg-surface-disabled-gray">
      <PointsHeader />
      <LigueLevels />
    </div>
  );
};

const ProfileTable: React.FC = () => {
  const { isConnected } = useAccount();

  if (!isConnected) return null;

  return (
    <div className="flex flex-col gap-4 mt-6">
      <p className="diatype-m-bold text-ink-primary-900">{m["points.profile.pointHistory"]()}</p>
      <PointsProfileTable />
    </div>
  );
};

const RewardsLoot: React.FC = () => {
  const { userIndex } = useAccount();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { nfts, unopenedCounts } = useBoxes({ pointsUrl, userIndex });
  const { oatStatuses } = useOats({ pointsUrl, userIndex });
  const { volume } = useUserPoints();

  return (
    <div className="p-4 lg:p-8 flex flex-col gap-5 lg:gap-8 bg-surface-primary-gray rounded-b-xl">
      <div className="p-4 lg:px-4 bg-surface-disabled-gray rounded-xl shadow-account-card">
        <PointsProgressBar currentVolume={volume} />
      </div>
      <BoxesSection unopenedBoxes={unopenedCounts} />
      <NFTsSection nfts={nfts} />
      <OATsSection oatStatuses={oatStatuses} />
    </div>
  );
};

const ProfileSection: React.FC = () => (
  <div className="flex flex-col gap-4 w-full">
    <ProfileHeader />
    <ProfileTable />
  </div>
);

const RewardsSection: React.FC = () => (
  <div className="bg-surface-disabled-gray rounded-xl shadow-account-card">
    <PointsHeader />
    <RewardsLoot />
  </div>
);

const LeaderboardSection: React.FC = () => (
  <div className="flex flex-col gap-8">
    <div className="bg-surface-disabled-gray rounded-xl shadow-account-card">
      <PointsHeader />
    </div>
    <div className="bg-surface-disabled-gray rounded-xl shadow-account-card">
      <LeaderboardTable />
    </div>
  </div>
);

const PointsCampaignTabs: React.FC = () => {
  const { activeTab, setActiveTab } = usePointsCampaign();

  return (
    <div className="flex flex-col w-full gap-8">
      <Tabs
        layoutId="points-campaign-tabs"
        selectedTab={activeTab}
        onTabChange={(value) => setActiveTab(value as PointsCampaignTab)}
        fullWidth
      >
        <Tab title="profile">
          <span className="flex items-center gap-1">
            <IconUser className="w-4 h-4" />
            {m["points.tabs.profile"]()}
          </span>
        </Tab>
        <Tab title="rewards">
          <span className="flex items-center gap-1">
            <IconGift className="w-4 h-4" />
            {m["points.tabs.rewards"]()}
          </span>
        </Tab>
        <Tab title="leaderboard">
          <span className="flex items-center gap-1">
            <IconStar className="w-4 h-4" />
            {m["points.tabs.leaderboard"]()}
          </span>
        </Tab>
      </Tabs>
      <div className={activeTab === "profile" ? "" : "hidden"}>
        <ProfileSection />
      </div>
      <div className={activeTab === "rewards" ? "" : "hidden"}>
        <RewardsSection />
      </div>
      <div className={activeTab === "leaderboard" ? "" : "hidden"}>
        <LeaderboardSection />
      </div>
    </div>
  );
};

export const PointsCampaign = Object.assign(PointsCampaignContainer, {
  Header: PointsCampaignHeader,
  Tabs: PointsCampaignTabs,
});
