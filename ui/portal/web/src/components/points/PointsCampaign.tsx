import { Button, ResizerContainer, Tab, Tabs, createContext } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

import type { PropsWithChildren } from "react";
import { PointsHeader } from "./PointsHeader";
import { UserPointsProvider } from "./useUserPoints";
import { LigueLevels, PointsProfileTable } from "./profile";
import {
  BoxCard,
  NFTCard,
  PointsProgressBar,
  ChestOpeningProvider,
  useChestOpening,
} from "./rewards";
import {
  ReferralStats,
  CommissionRates,
  MyCommission,
  ReferralFaqs,
  type ReferralMode,
} from "./referral";

type PointsCampaignTab = "profile" | "rewards" | "referral";

const [PointsCampaignProvider, usePointsCampaign] = createContext<{
  activeTab: PointsCampaignTab;
  setActiveTab: (tab: PointsCampaignTab) => void;
}>({
  name: "PointsCampaignContext",
});

const PointsCampaignContainer: React.FC<PropsWithChildren> = ({ children }) => {
  const [activeTab, setActiveTab] = useState<PointsCampaignTab>("profile");

  return (
    <UserPointsProvider>
      <ChestOpeningProvider>
        <PointsCampaignProvider value={{ activeTab, setActiveTab }}>
          <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
            <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
              {children}
            </div>
          </div>
        </PointsCampaignProvider>
      </ChestOpeningProvider>
    </UserPointsProvider>
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
  const currentVolume = 490000;
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
          <NFTCard rarity="common" quantity={4} imageSrc="/images/points/nft/common.png" />
          <NFTCard rarity="uncommon" quantity={2} imageSrc="/images/points/nft/uncommon.png" />
          <NFTCard rarity="epic" quantity={2} imageSrc="/images/points/nft/epic.png" />
          <NFTCard rarity="golden" quantity={2} imageSrc="/images/points/nft/mythic.png" />
          <NFTCard rarity="legendary" quantity={2} imageSrc="/images/points/nft/legendary.png" />
          <NFTCard rarity="rare" quantity={2} imageSrc="/images/points/nft/rare.png" />
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

const ReferralSection: React.FC = () => {
  const [referralMode, setReferralMode] = useState<ReferralMode>("affiliate");

  return (
    <div className="flex flex-col gap-6 w-full">
      <ResizerContainer
        layoutId="referra-container"
        className="bg-surface-disabled-gray rounded-xl shadow-account-card overflow-hidden relative"
      >
        <PointsHeader />
        <ResizerContainer
          layoutId="referral-stats"
          className="p-4 lg:p-8 bg-surface-primary-gray rounded-b-xl flex flex-col gap-6"
        >
          <ReferralStats mode={referralMode} onModeChange={setReferralMode} />
          {referralMode === "affiliate" && <CommissionRates />}
        </ResizerContainer>
      </ResizerContainer>
      <MyCommission mode={referralMode} />
      <ReferralFaqs />
    </div>
  );
};

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
        <Tab title="referral">Referral</Tab>
      </Tabs>
      {activeTab === "profile" ? <ProfileSection /> : null}
      {activeTab === "rewards" ? <RewardsSection /> : null}
      {activeTab === "referral" ? <ReferralSection /> : null}
    </div>
  );
};

export const PointsCampaign = Object.assign(PointsCampaignContainer, {
  Header: PointsCampaignHeader,
  Tabs: PointsCampaignTabs,
});
