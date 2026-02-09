import {
  Badge,
  Button,
  IconEdit,
  IconLink,
  IconUser,
  Input,
  Modals,
  ProgressBar,
  Tab,
  Tabs,
  TextCopy,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";

type ReferralMode = "affiliate" | "trader";

type ReferralStatsProps = {
  mode: ReferralMode;
  onModeChange: (mode: ReferralMode) => void;
};

const UNLOCK_VOLUME = 10000;
const TIER_2_VOLUME = 100000;

const AffiliateStats: React.FC = () => {
  const { showModal } = useApp();
  const currentVolume = 11000;
  const isUnlocked = currentVolume >= UNLOCK_VOLUME;
  const targetVolume = isUnlocked ? TIER_2_VOLUME : UNLOCK_VOLUME;
  const progress = Math.min((currentVolume / targetVolume) * 100, 100);
  const remaining = targetVolume - currentVolume;

  const referralLink = "https://pr...12345";
  const referralCode = "123451";

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="w-full rounded-xl bg-surface-disabled-gray p-4 lg:p-6 flex flex-col gap-6 shadow-account-card relative">
        <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-between">
          <div className="flex flex-col items-center lg:items-start">
            <div className="flex items-center gap-1">
              <p className="text-primitives-warning-500 h3-bold">10% / 5%</p>
              <IconEdit
                className="w-6 h-6 text-fg-secondary-500 mb-1 hover:text-ink-secondary-blue cursor-pointer"
                onClick={() => showModal(Modals.EditCommissionRate)}
              />
            </div>
            <p className="text-ink-tertiary-500 diatype-m-medium">Commission Rate (You/ Referee)</p>
          </div>
          <div className="flex flex-col items-center">
            <p className="text-ink-primary-900 h3-bold">$320.50</p>
            <p className="text-ink-tertiary-500 diatype-m-medium">Total Commission</p>
          </div>
          <div className="flex flex-col items-center lg:items-end">
            <p className="text-primitives-warning-500 h3-bold">$1,450.00</p>
            <p className="text-ink-tertiary-500 diatype-m-medium">Total Referral Volume</p>
          </div>
        </div>

        <ProgressBar
          progress={progress}
          leftLabel={`$${remaining.toLocaleString()} volume until Tier 2`}
          rightLabel={`$${(targetVolume / 1000).toFixed(0)}K`}
          thumbSrc="/images/points/pointBarThumb.png"
          classNames={{
            leftLabel: "diatype-s-medium",
            rightLabel: "diatype-m-bold text-primitives-warning-500",
          }}
        />

        <div className="flex flex-col gap-4">
          <div className="flex flex-col lg:flex-row gap-4">
            <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">Total Referees</p>
              <p className="text-ink-primary-900 diatype-m-bold">50</p>
            </div>
            <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">Total Active Referees</p>
              <p className="text-ink-primary-900 diatype-m-bold">50</p>
            </div>
          </div>

          {isUnlocked ? (
            <div className="flex flex-col lg:flex-row gap-4">
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">My Referral Link</p>
                <div className="flex items-center gap-2">
                  <p className="text-ink-primary-900 diatype-m-bold">{referralLink}</p>
                  <TextCopy
                    copyText={referralLink}
                    className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                  />
                </div>
              </div>
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">My Referral Code</p>
                <div className="flex items-center gap-2">
                  <p className="text-ink-primary-900 diatype-m-bold">{referralCode}</p>
                  <TextCopy copyText={referralCode} className="w-5 h-5 text-ink-tertiary-500" />
                  <IconEdit className="w-5 h-5 text-ink-tertiary-500 hover:text-ink-secondary-blue cursor-pointer" />
                </div>
              </div>
            </div>
          ) : (
            <div className="min-h-[280px] lg:min-h-[180px] mt-4">
              <div className="relative z-10 flex flex-col gap-4 lg:max-w-sm">
                <div className="flex flex-col gap-2">
                  <h3 className="display-heading-xs text-ink-primary-900 max-w-sm">
                    Unlock your referral code with $10k volume!
                  </h3>
                  <p className="text-ink-tertiary-500 diatype-m-regular max-w-sm">
                    Invite your friends and earn up to{" "}
                    <span className="text-utility-success-500 font-bold">30%</span> commission.
                  </p>
                </div>
                <Button variant="primary" size="sm">
                  Trade now
                </Button>
              </div>
              <img
                src="/images/points/referral-banner.png"
                alt="Referral banner"
                className="absolute bottom-0 right-1/2 translate-x-1/2 lg:right-[3rem] lg:translate-x-0 w-[200px] lg:w-auto h-auto object-contain pointer-events-none"
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const TraderStats: React.FC = () => {
  const [referralCode, setReferralCode] = useState("");
  const hasReferrer = true; // Mock: change to false to see the input view
  const referrerName = "Scranton";

  return (
    <div className={twMerge("w-full flex flex-col gap-6", !hasReferrer && "pb-[153px] lg:pb-0")}>
      <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-between">
        <div className="flex flex-col items-center lg:items-start">
          <p className="text-utility-warning-600 h3-bold">15%</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Rebate Rate</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-utility-warning-600 h3-bold">$320.50</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Total Rebates</p>
        </div>
        <div className="flex flex-col items-center lg:items-end">
          <p className="text-utility-warning-600 h3-bold">$1,450.00</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Total Trading Volume</p>
        </div>
      </div>

      {hasReferrer ? (
        <div className="w-full rounded-xl bg-surface-tertiary-gray px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <IconUser className="w-5 h-5 text-primitives-blue-light-400" />
            <p className="text-ink-primary-900 diatype-m-medium">Your Referrer</p>
          </div>
          <Badge text={referrerName} color="blue" />
        </div>
      ) : (
        <>
          <div className="w-full h-px bg-outline-secondary-gray" />
          <div className="min-h-[280px] lg:min-h-[180px]">
            <div className="relative z-10 flex flex-col gap -8 lg:max-w-sm">
              <div className="flex flex-col gap-2">
                <h3 className="display-heading-xs text-ink-primary-900 max-w-sm">
                  Refer friends and unlock more rewards together!
                </h3>
                <p className="text-ink-tertiary-500 diatype-m-regular max-w-sm">
                  Get up to <span className="text-utility-success-500 font-bold">15%</span> fee
                  rebates by submitting your friend's referral code!
                </p>
              </div>
              <Input
                label="Referral Code"
                value={referralCode}
                onChange={(e) => setReferralCode(e.target.value)}
                placeholder="Enter your friend's referral code"
                endContent={
                  <Button variant="link" className="p-0">
                    Submit
                  </Button>
                }
              />
            </div>
            <img
              src="/images/characters/friends.svg"
              alt="Refer friends"
              className="absolute bottom-[-5rem] lg:bottom-[-6rem] right-1/2 translate-x-1/2 lg:right-[3rem] lg:translate-x-0 w-[260px] lg:w-[320px] h-auto object-contain pointer-events-none"
            />
          </div>
        </>
      )}
    </div>
  );
};

export const ReferralStats: React.FC<ReferralStatsProps> = ({ mode, onModeChange }) => {
  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="flex justify-center">
        <Tabs
          layoutId="referral-mode-tabs"
          fullWidth
          selectedTab={mode}
          onTabChange={(value) => onModeChange(value as ReferralMode)}
        >
          <Tab title="affiliate">
            <span className="flex items-center gap-2">
              Affiliate <Badge text="Tier 1" color="rice" />
            </span>
          </Tab>
          <Tab title="trader">Trader</Tab>
        </Tabs>
      </div>

      {mode === "affiliate" ? <AffiliateStats /> : <TraderStats />}
    </div>
  );
};

export type { ReferralMode };
