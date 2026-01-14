import { Button, Tab, Tabs, createContext } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

import type { PropsWithChildren } from "react";
import { PointsProfileTable } from "./PointsProfileTable";
import { PointsHeader } from "./PointsHeader";
import { BoxCard } from "./BoxCard";

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
      <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
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
      <PointsHeader />
      <img
        src="/images/banner-points-dango.png"
        alt="Points Banner"
        className="w-full min-h-[25rem] drag-none select-none object-cover"
      />
    </div>
  );
};

const ProfileTable: React.FC = () => {
  return (
    <div className="bg-surface-primary-gray rounded-xl shadow-account-card">
      <div className="px-6 py-4">
        <p className="diatype-m-bold text-ink-primary-900">Point History</p>
      </div>
      <PointsProfileTable />
      <div className="px-6 py-4 flex items-center justify-center">
        <Button>Get Started!</Button>
      </div>
    </div>
  );
};

const RewardsLoot: React.FC = () => {
  return (
    <div className="p-5 lg:p-8 flex flex-col gap-5 lg:gap-8 bg-surface-tertiary-gray rounded-b-xl">
      <div className="p-4 lg:px-8 bg-surface-disabled-gray rounded-xl">Progress bar</div>
      <div className="flex flex-col gap-3">
        <p className="h3-bold text-ink-primary-900">My boxes</p>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
          <BoxCard variant="bronze" />
          <BoxCard variant="silver" />
          <BoxCard variant="gold" lock={true} />
          <BoxCard variant="crystal" lock={true} />
        </div>
      </div>
      <div className="flex flex-col gap-3">
        <p className="h3-bold text-ink-primary-900">My NFTs</p>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 lg:gap-8">
          <p>NFT</p>
          <p>NFT</p>
          <p>NFT</p>
          <p>NFT</p>
        </div>
      </div>
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
  <div className="bg-surface-primary-gray rounded-xl shadow-account-card">
    <PointsHeader />
    <RewardsLoot />
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
