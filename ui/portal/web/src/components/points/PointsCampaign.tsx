import {
  Button,
  IconFriendshipGroup,
  IconInfo,
  IconSprout,
  IconSwapMoney,
  Tab,
  Tabs,
  createContext,
} from "@left-curve/applets-kit";
import { useState } from "react";

import type { PropsWithChildren } from "react";

type PointsCampaignTab = "profile" | "rewards";

const [PointsCampaignProvider, usePointsCampaign] = createContext<{
  activeTab: PointsCampaignTab;
  setActiveTab: (tab: PointsCampaignTab) => void;
}>({
  name: "PointsCampaignContext",
});

const PointsCampaignContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const [activeTab, setActiveTab] = useState<PointsCampaignTab>("profile");

  return (
    <PointsCampaignProvider value={{ activeTab, setActiveTab }}>
      <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit">
        <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
          {children}
        </div>
      </div>
    </PointsCampaignProvider>
  );
};

const PointsCampaignHeader: React.FC = () => (
  <div className="flex flex-col gap-8 w-full items-center">
    <div className="max-w-[15.5rem] flex flex-col gap-2 items-center text-center">
      <p className="text-ink-tertiary-500 diatype-m-regular">Welcome to Dango's</p>
      <h1 className="exposure-h1-italic lg:text-[48px] text-ink-primary-rice">POINTS PROGRAM</h1>
    </div>
    <Button variant="utility">Read Rules</Button>
  </div>
);

const ProfileHeader: React.FC = () => {
  return (
    <div className="w-full rounded-xl shadow-account-card overflow-hidden bg-surface-disabled-gray">
      <div className="p-4 lg:p-8 lg:pb-[30px] flex flex-col gap-4">
        <div className="w-full rounded-xl bg-surface-tertiary-gray p-4 flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
          <div className="flex flex-col items-center">
            <p className="text-ink-secondary-rice h3-bold">16.300</p>
            <p className="text-ink-tertiary-500 diatype-m-medium">My points</p>
          </div>
          <div className="flex flex-col items-center">
            <p className="text-ink-secondary-rice h3-bold">$75,000</p>
            <p className="text-ink-tertiary-500 diatype-m-medium">My volume</p>
          </div>
          <div className="flex flex-col items-center">
            <p className="text-ink-secondary-rice h3-bold">#11,200</p>
            <p className="text-ink-tertiary-500 diatype-m-medium">My rank</p>
          </div>
        </div>
        <div className="flex flex-col lg:flex-row gap-4 w-full">
          <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
            <IconSwapMoney />
            <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
              <p className="text-ink-primary-900">3.000</p>
              <p>Points</p>
              <IconInfo />
            </div>
          </div>
          <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
            <IconSprout />
            <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
              <p className="text-ink-primary-900">12.000</p>
              <p>Points</p>
              <IconInfo />
            </div>
          </div>
          <div className="bg-surface-tertiary-gray px-3 py-2 flex items-center justify-between rounded-xl flex-1">
            <IconFriendshipGroup />
            <div className="flex items-center gap-1 text-ink-tertiary-500 diatype-m-medium">
              <p className="text-ink-primary-900">8.500</p>
              <p>Points</p>
              <IconInfo />
            </div>
          </div>
        </div>
      </div>
      <img
        src="/images/banner-points-dango.png"
        alt="Points Banner"
        className="w-full min-h-[25rem] drag-none select-none"
      />
    </div>
  );
};

const ProfileTable: React.FC = () => <div>Profile table</div>;

const RewardsTable: React.FC = () => <div>Rewards table</div>;

const ProfileSection: React.FC = () => (
  <div className="flex flex-col gap-4 w-full">
    <ProfileHeader />
    <ProfileTable />
  </div>
);

const RewardsSection: React.FC = () => (
  <div className="flex flex-col gap-4 w-full">
    <RewardsTable />
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
        <Tab title="profile">Profile</Tab>
        <Tab title="rewards">Rewards</Tab>
      </Tabs>
      {activeTab === "profile" ? <ProfileSection /> : null}
      {activeTab === "rewards" ? <RewardsSection /> : null}
    </div>
  );
};

export const PointsCampaign = Object.assign(PointsCampaignContainer, {
  Header: PointsCampaignHeader,
  Tabs: PointsCampaignTabs,
});
