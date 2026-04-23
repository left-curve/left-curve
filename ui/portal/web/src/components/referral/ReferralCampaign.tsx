import { Badge, Button, ResizerContainer, Tab, Tabs, createContext } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";
import type { PropsWithChildren } from "react";

import { MobileTitle } from "../foundation/MobileTitle";
import {
  AffiliateStats,
  CommissionRates,
  MyCommission,
  ReferralFaqs,
  TraderStats,
  type ReferralMode,
} from "../points/referral";

const [ReferralCampaignProvider, useReferralCampaign] = createContext<{
  activeTab: ReferralMode;
  setActiveTab: (tab: ReferralMode) => void;
}>({
  name: "ReferralCampaignContext",
});

type ReferralCampaignContainerProps = PropsWithChildren<{
  activeTab: ReferralMode;
  onTabChange: (tab: ReferralMode) => void;
}>;

const ReferralCampaignContainer: React.FC<ReferralCampaignContainerProps> = ({
  children,
  activeTab,
  onTabChange,
}) => {
  return (
    <ReferralCampaignProvider value={{ activeTab, setActiveTab: onTabChange }}>
      <div className="w-full md:max-w-[56.125rem] mx-auto flex flex-col p-4 lg:p-0 lg:pt-6 lg:pb-20 pt-6 gap-4 min-h-[100svh] md:min-h-fit pb-20">
        <MobileTitle title={m["referral.mobileTitle"]()} />
        <div className="pt-10 lg:pt-20 gap-[60px] flex flex-col items-center justify-center relative">
          {children}
        </div>
      </div>
    </ReferralCampaignProvider>
  );
};

const ReferralCampaignHeader: React.FC = () => (
  <div className="flex flex-col gap-8 w-full items-center relative">
    <div className="absolute top-[60%] left-1/2 -translate-x-1/2 -translate-y-1/2 w-[160rem] h-[100rem] bg-[radial-gradient(ellipse,rgba(245,221,184,0.6)_0%,rgba(245,221,184,0.2)_40%,transparent_70%)] pointer-events-none" />
    <div className="max-w-[15.5rem] flex flex-col gap-2 items-center text-center relative z-10">
      <p className="text-ink-tertiary-500 diatype-m-regular">{m["referral.welcome"]()}</p>
      <h1 className="exposure-h1-italic lg:text-[48px] text-ink-primary-rice">
        {m["referral.title"]()}
      </h1>
    </div>
    <Button
      variant="utility"
      onClick={() => window.open("https://dango-4.gitbook.io/dango-docs/referral-system")}
    >
      {m["referral.readRules"]()}
    </Button>
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
  const { activeTab, setActiveTab } = useReferralCampaign();

  return (
    <div className="flex flex-col w-full gap-8">
      <Tabs
        layoutId="referral-campaign-tabs"
        selectedTab={activeTab}
        onTabChange={(value) => setActiveTab(value as ReferralMode)}
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
        {activeTab === "affiliate" ? <AffiliateSection /> : <TraderSection />}
        <MyCommission mode={activeTab} />
        {activeTab === "affiliate" && <ReferralFaqs />}
      </div>
    </div>
  );
};

export const ReferralCampaign = Object.assign(ReferralCampaignContainer, {
  Header: ReferralCampaignHeader,
  Content: ReferralCampaignContent,
});
