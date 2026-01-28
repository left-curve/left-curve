import { Button, Tab, Tabs, createContext } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

import type { PropsWithChildren } from "react";
import { PointsProfileTable } from "./PointsProfileTable";
import { PointsHeader } from "./PointsHeader";
import { BoxCard } from "./BoxCard";
import { LigueLevels } from "./LigueLevels";
import { NFTCard } from "./NFTCard";
import { PointsProgressBar } from "./PointsProgressBar";
import { ChestOpeningProvider, useChestOpening } from "./useChestOpening";

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
    <ChestOpeningProvider>
      <PointsCampaignProvider value={{ activeTab, setActiveTab }}>
        <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
          <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
            {children}
          </div>
        </div>
      </PointsCampaignProvider>
    </ChestOpeningProvider>
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
      <LigueLevels />
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
  const currentVolume = 500350;
  const { openChest } = useChestOpening();

  return (
    <div className="p-5 lg:p-8 flex flex-col gap-5 lg:gap-8 bg-surface-tertiary-gray rounded-b-xl">
      <div className="p-4 lg:px-8 bg-surface-disabled-gray rounded-xl">
        <PointsProgressBar currentVolume={currentVolume} />
      </div>
      <div className="flex flex-col gap-3">
        <p className="h3-bold text-ink-primary-900">My boxes</p>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
          <BoxCard variant="bronze" volume={currentVolume} onClick={() => openChest("bronze")} />
          <BoxCard variant="silver" volume={currentVolume} onClick={() => openChest("silver")} />
          <BoxCard variant="gold" volume={currentVolume} onClick={() => openChest("gold")} />
          <BoxCard variant="crystal" volume={currentVolume} onClick={() => openChest("crystal")} />
        </div>
      </div>
      <div className="flex flex-col gap-3">
        <p className="h3-bold text-ink-primary-900">My NFTs</p>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 lg:gap-8">
          <NFTCard
            rarity="common"
            quantity={4}
            imageSrc="https://www.figma.com/api/mcp/asset/c3b5358b-c2b3-4bc0-a1d6-30117c53b423"
          />
          <NFTCard
            rarity="uncommon"
            quantity={2}
            imageSrc="https://www.figma.com/api/mcp/asset/9680ed08-69ef-471f-83a5-813846101610"
          />
          <NFTCard
            rarity="epic"
            quantity={2}
            imageSrc="https://www.figma.com/api/mcp/asset/fe211f83-4040-4023-ae76-da8967f68d53"
          />
          <NFTCard
            rarity="golden"
            quantity={2}
            imageSrc="https://www.figma.com/api/mcp/asset/21afd773-9bb9-4cd4-a30c-dc9a356d0708"
          />
          <NFTCard
            rarity="legendary"
            quantity={2}
            imageSrc="https://www.figma.com/api/mcp/asset/e21cd2e1-7549-4b53-8916-3d1713bb5699"
          />
          <NFTCard
            rarity="rare"
            quantity={2}
            imageSrc="https://www.figma.com/api/mcp/asset/555ebcef-e6f2-490b-9c44-b72b36dad681"
          />
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
