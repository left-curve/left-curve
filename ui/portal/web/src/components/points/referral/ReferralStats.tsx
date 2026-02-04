import { Button, IconLink, ProgressBar, Tab, Tabs, TextCopy } from "@left-curve/applets-kit";
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
  const currentVolume = 5000;
  const isUnlocked = currentVolume >= UNLOCK_VOLUME;
  const targetVolume = isUnlocked ? TIER_2_VOLUME : UNLOCK_VOLUME;
  const progress = Math.min((currentVolume / targetVolume) * 100, 100);
  const remaining = targetVolume - currentVolume;

  const referralLink = "https://pr...12345";
  const referralCode = "123451";

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="w-full rounded-xl bg-surface-disabled-gray p-4 lg:p-6 flex flex-col gap-6 shadow-account-card">
        <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
          <div className="flex flex-col items-center lg:items-start">
            <div className="flex items-center gap-1">
              <p className="text-primitives-warning-500 h3-bold">10% / 5%</p>
              <IconLink className="w-4 h-4 text-primitives-warning-500" />
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

        <div className="flex flex-col lg:flex-row gap-4">
          <div className="flex-1 bg-surface-primary-rice rounded-xl p-4 flex justify-between items-center">
            <p className="text-ink-tertiary-500 diatype-m-medium">Total Referees</p>
            <p className="text-ink-primary-900 diatype-m-bold">50</p>
          </div>
          <div className="flex-1 bg-surface-primary-rice rounded-xl p-4 flex justify-between items-center">
            <p className="text-ink-tertiary-500 diatype-m-medium">Total Active Referees</p>
            <p className="text-ink-primary-900 diatype-m-bold">50</p>
          </div>
        </div>

        {isUnlocked ? (
          <div className="flex flex-col lg:flex-row gap-4">
            <div className="flex-1 bg-surface-primary-rice rounded-xl p-4 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">My Referral Link</p>
              <div className="flex items-center gap-2">
                <p className="text-ink-primary-900 diatype-m-bold">{referralLink}</p>
                <TextCopy
                  copyText={referralLink}
                  className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                />
              </div>
            </div>
            <div className="flex-1 bg-surface-primary-rice rounded-xl p-4 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">My Referral Code</p>
              <div className="flex items-center gap-2">
                <p className="text-ink-primary-900 diatype-m-bold">{referralCode}</p>
                <TextCopy
                  copyText={referralCode}
                  className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                />
                <IconLink className="w-4 h-4 cursor-pointer text-ink-tertiary-500" />
              </div>
            </div>
          </div>
        ) : (
          <div className="w-full rounded-xl bg-surface-disabled-gray p-6 relative overflow-hidden min-h-[180px]">
            <div className="relative z-10 max-w-[60%] lg:max-w-[50%]">
              <h3 className="exposure-m-italic text-ink-primary-900">
                Unlock your referral code with{" "}
                <span className="text-primitives-warning-500">$10k</span> volume!
              </h3>
              <p className="text-ink-tertiary-500 diatype-m-regular mt-2">
                Invite your friends and earn up to{" "}
                <span className="text-primitives-warning-500 font-bold">30%</span> commission.
              </p>
              <Button variant="primary" className="mt-4">
                Trade now
              </Button>
            </div>
            <img
              src="/images/points/referral-banner.png"
              alt="Referral banner"
              className="absolute bottom-0 right-0 w-[200px] lg:w-[280px] h-auto object-contain pointer-events-none"
            />
          </div>
        )}
      </div>
    </div>
  );
};

const TraderStats: React.FC = () => {
  const [referralCode, setReferralCode] = useState("");

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="w-full rounded-xl bg-surface-tertiary-rice border border-outline-primary-gray p-4 flex flex-col gap-4 items-center lg:flex-row lg:justify-around">
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">15%</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Rebate Rate</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">$320.50</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Total Rebates</p>
        </div>
        <div className="flex flex-col items-center">
          <p className="text-ink-secondary-rice h3-bold">$1,450.00</p>
          <p className="text-ink-tertiary-500 diatype-m-medium">Total Trading Volume</p>
        </div>
      </div>

      <div className="w-full rounded-xl bg-gradient-to-r from-purple-50 to-indigo-50 border border-outline-primary-gray p-6 flex flex-col lg:flex-row gap-6 items-center">
        <div className="flex-1">
          <img
            src="/images/referral/trader-banner.png"
            alt="Trader banner"
            className="w-[200px] h-[200px] object-contain mx-auto"
          />
        </div>
        <div className="flex-1 flex flex-col gap-4">
          <div>
            <h3 className="exposure-m-italic text-ink-primary-900">
              Refer friends and unlock more rewards together!
            </h3>
            <p className="text-ink-tertiary-500 diatype-m-regular mt-2">
              Get up to 15% fee rebate by submitting your friend's referral code.
            </p>
          </div>
          <div className="flex flex-col gap-2">
            <p className="text-ink-tertiary-500 diatype-s-medium">Referral Code</p>
            <div className="flex gap-2">
              <input
                type="text"
                value={referralCode}
                onChange={(e) => setReferralCode(e.target.value)}
                placeholder="Enter your Friend's referral code"
                className="flex-1 px-4 py-2 rounded-lg border border-outline-primary-gray bg-surface-primary-gray text-ink-primary-900 diatype-m-regular placeholder:text-ink-tertiary-500"
              />
              <Button variant="secondary">Submit</Button>
            </div>
          </div>
        </div>
      </div>
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
          <Tab title="affiliate">Affiliate</Tab>
          <Tab title="trader">Trader</Tab>
        </Tabs>
      </div>

      {mode === "affiliate" ? <AffiliateStats /> : <TraderStats />}
    </div>
  );
};

export type { ReferralMode };
