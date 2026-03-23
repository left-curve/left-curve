import {
  Badge,
  Button,
  ResizerContainer,
  Tab,
  Tabs,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";
import { useState } from "react";
import type { PropsWithChildren } from "react";

import {
  AffiliateStats,
  CommissionRates,
  MyCommission,
  ReferralFaqs,
  TraderStats,
  type ReferralMode,
} from "../points/referral";

const ReferralCampaignContainer: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
      <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
        {children}
      </div>
    </div>
  );
};

const ReferralCampaignHeader: React.FC = () => (
  <div className="flex flex-col gap-8 w-full items-center">
    <div className="max-w-[15.5rem] flex flex-col gap-2 items-center text-center">
      <p className="text-ink-tertiary-500 diatype-m-regular">{m["referral.welcome"]()}</p>
      <h1 className="exposure-h1-italic lg:text-[48px] text-ink-primary-rice">{m["referral.title"]()}</h1>
    </div>
    <Button variant="utility">{m["referral.readRules"]()}</Button>
  </div>
);

const AffiliateSection: React.FC = () => (
  <ResizerContainer
    layoutId="referral-container"
    className="bg-surface-disabled-gray rounded-xl shadow-account-card overflow-hidden relative"
  >
    <ResizerContainer
      layoutId="referral-stats"
      className="p-4 lg:p-8 bg-surface-primary-gray rounded-xl flex flex-col gap-6"
    >
      <AffiliateStats />
      <CommissionRates />
    </ResizerContainer>
  </ResizerContainer>
);

const TraderSection: React.FC = () => (
  <ResizerContainer
    layoutId="referral-container"
    className="bg-surface-disabled-gray rounded-xl shadow-account-card overflow-hidden relative"
  >
    <ResizerContainer
      layoutId="referral-stats"
      className="p-4 lg:p-8 bg-surface-primary-gray rounded-xl flex flex-col gap-6"
    >
      <TraderStats />
    </ResizerContainer>
  </ResizerContainer>
);

const ReferralCampaignContent: React.FC = () => {
  const [referralMode, setReferralMode] = useState<ReferralMode>("affiliate");

  return (
    <div className="flex flex-col w-full gap-8">
      <Tabs
        layoutId="referral-campaign-tabs"
        selectedTab={referralMode}
        onTabChange={(value) => setReferralMode(value as ReferralMode)}
        fullWidth
      >
        <Tab title="affiliate">
          <span className="flex items-center gap-2">
            {m["referral.affiliate"]()} <Badge text="Tier 1" color="rice" />
          </span>
        </Tab>
        <Tab title="trader">{m["referral.trader"]()}</Tab>
      </Tabs>
      <div className="flex flex-col gap-6 w-full">
        {referralMode === "affiliate" ? <AffiliateSection /> : <TraderSection />}
        <MyCommission mode={referralMode} />
        {referralMode === "affiliate" && <ReferralFaqs />}
      </div>
    </div>
  );
};

export const ReferralCampaign = Object.assign(ReferralCampaignContainer, {
  Header: ReferralCampaignHeader,
  Content: ReferralCampaignContent,
});
