import {
  Badge,
  Button,
  IconEdit,
  IconUser,
  Input,
  Modals,
  ProgressBar,
  Skeleton,
  Tab,
  Tabs,
  TextCopy,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  useAccount,
  useReferrer,
  useReferralData,
  useReferralSettings,
  useUserVolume,
  getReferralCode,
  getReferralLink,
} from "@left-curve/store";
import type React from "react";
import { useState } from "react";

type ReferralMode = "affiliate" | "trader";

type ReferralStatsProps = {
  mode: ReferralMode;
  onModeChange: (mode: ReferralMode) => void;
};

const UNLOCK_VOLUME = 10000;
const TIER_2_VOLUME = 100000;

/**
 * Format a number as USD currency
 */
const formatUSD = (value: number | string): string => {
  const num = typeof value === "string" ? Number(value) : value;
  if (Number.isNaN(num)) return "$0.00";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num);
};

/**
 * Format a percentage (e.g., "0.1" -> "10%")
 */
const formatPercent = (value: string | undefined): string => {
  if (!value) return "0%";
  const num = Number(value);
  if (Number.isNaN(num)) return "0%";
  return `${(num * 100).toFixed(0)}%`;
};

/**
 * Truncate a URL for display
 */
const truncateUrl = (url: string, maxLength = 20): string => {
  if (url.length <= maxLength) return url;
  const start = url.slice(0, maxLength - 5);
  return `${start}...`;
};

export const AffiliateStats: React.FC = () => {
  const { showModal } = useApp();
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  // Fetch real data from contract (only when connected)
  const { volume, isLoading: volumeLoading } = useUserVolume({
    userIndex,
    days: 30,
  });
  const { referralData, isLoading: dataLoading } = useReferralData({
    userIndex,
  });
  const { settings, isLoading: settingsLoading } = useReferralSettings({
    userIndex,
  });

  const isLoading = isConnected && (volumeLoading || dataLoading || settingsLoading);

  // Derive values from fetched data
  const currentVolume = volume ?? 0;
  const isUnlocked = isConnected && currentVolume >= UNLOCK_VOLUME;
  const targetVolume = isUnlocked ? TIER_2_VOLUME : UNLOCK_VOLUME;
  const progress = isConnected ? Math.min((currentVolume / targetVolume) * 100, 100) : 0;
  const remaining = Math.max(targetVolume - currentVolume, 0);

  // Referral code and link derived from user index
  const referralCode = getReferralCode(userIndex);
  const referralLink = getReferralLink(userIndex);
  const truncatedLink = truncateUrl(referralLink);

  // Commission rates from settings
  const commissionRate = settings?.commission_rebound ?? "0";
  const shareRatio = settings?.share_ratio ?? "0";
  const rateDisplay = isConnected
    ? `${formatPercent(commissionRate)} / ${formatPercent(shareRatio)}`
    : "-- / --";

  // Referral data
  const totalCommission = referralData?.commission ?? "0";
  const totalVolume = referralData?.volume ?? "0";
  const totalReferees = referralData?.active_referees ?? 0;

  // Progress bar labels
  const progressLeftLabel = isConnected
    ? m["referral.stats.volumeUntilTier2"]({ amount: formatUSD(remaining) })
    : m["referral.stats.notLoggedIn"]();
  const progressRightLabel = `$${(targetVolume / 1000).toFixed(0)}K`;

  return (
    <div className="flex flex-col gap-4 w-full">
      <div className="w-full rounded-xl bg-surface-disabled-gray p-4 lg:p-6 flex flex-col gap-6 shadow-account-card relative">
        <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-between">
          <div className="flex flex-col items-center lg:items-start">
            <div className="flex items-center gap-1">
              {isLoading ? (
                <Skeleton className="w-24 h-8" />
              ) : (
                <p className="text-primitives-warning-500 h3-bold">{rateDisplay}</p>
              )}
              {isConnected && (
                <IconEdit
                  className="w-6 h-6 text-fg-secondary-500 mb-1 hover:text-ink-secondary-blue cursor-pointer"
                  onClick={() => showModal(Modals.EditCommissionRate)}
                />
              )}
            </div>
            <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.commissionRate"]()}</p>
          </div>
          <div className="flex flex-col items-center">
            {isLoading ? (
              <Skeleton className="w-24 h-8" />
            ) : (
              <p className="text-ink-primary-900 h3-bold">
                {isConnected ? formatUSD(totalCommission) : "--"}
              </p>
            )}
            <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalCommissionPoints"]()}</p>
          </div>
          <div className="flex flex-col items-center lg:items-end">
            {isLoading ? (
              <Skeleton className="w-24 h-8" />
            ) : (
              <p className="text-primitives-warning-500 h3-bold">
                {isConnected ? formatUSD(totalVolume) : "--"}
              </p>
            )}
            <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalReferralVolume"]()}</p>
          </div>
        </div>

        <ProgressBar
          progress={progress}
          leftLabel={progressLeftLabel}
          rightLabel={progressRightLabel}
          thumbSrc="/images/points/pointBarThumb.png"
          classNames={{
            leftLabel: "diatype-s-medium",
            rightLabel: "diatype-m-bold text-primitives-warning-500",
          }}
        />

        <div className="flex flex-col gap-4">
          <div className="flex flex-col lg:flex-row gap-4">
            <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalReferees"]()}</p>
              {isLoading ? (
                <Skeleton className="w-12 h-6" />
              ) : (
                <p className="text-ink-primary-900 diatype-m-bold">
                  {isConnected ? totalReferees : "--"}
                </p>
              )}
            </div>
            <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalActiveReferees"]()}</p>
              {isLoading ? (
                <Skeleton className="w-12 h-6" />
              ) : (
                <p className="text-ink-primary-900 diatype-m-bold">
                  {isConnected ? totalReferees : "--"}
                </p>
              )}
            </div>
          </div>

          {isUnlocked ? (
            <div className="flex flex-col lg:flex-row gap-4">
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.myReferralLink"]()}</p>
                <div className="flex items-center gap-2">
                  <p className="text-ink-primary-900 diatype-m-bold">{truncatedLink}</p>
                  <TextCopy
                    copyText={referralLink}
                    className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                  />
                </div>
              </div>
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.myReferralCode"]()}</p>
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
                    {m["referral.affiliateSection.unlockTitle"]()}
                  </h3>
                  <p className="text-ink-tertiary-500 diatype-m-regular max-w-sm">
                    {m["referral.affiliateSection.unlockDescription"]({ percent: "30%" })}
                  </p>
                </div>
                {isConnected ? (
                  <Button variant="primary" size="sm">
                    {m["referral.affiliateSection.tradeNow"]()}
                  </Button>
                ) : (
                  <Button variant="primary" size="sm" onClick={() => showModal(Modals.Login)}>
                    {m["referral.affiliateSection.logIn"]()}
                  </Button>
                )}
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

export const TraderStats: React.FC = () => {
  const { showModal } = useApp();
  const [referralCodeInput, setReferralCodeInput] = useState("");
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  // Fetch referrer and volume data (only when connected)
  const { referrer, hasReferrer, isLoading: referrerLoading } = useReferrer({
    userIndex,
  });
  const { volume, isLoading: volumeLoading } = useUserVolume({
    userIndex,
    days: 30,
  });
  const { settings, isLoading: settingsLoading } = useReferralSettings({
    userIndex: referrer ?? undefined,
    enabled: hasReferrer,
  });

  const isLoading = isConnected && (referrerLoading || volumeLoading || settingsLoading);

  // Rebate rate from referrer's settings (share_ratio is what referee gets)
  const rebateRate = settings?.share_ratio ?? "0";
  // TODO: Get actual rebate totals from contract when available
  const totalRebates = "0";
  const totalVolume = volume ?? 0;

  // Display referrer as user index (could be enhanced to show username)
  const referrerDisplay = referrer ? `#${referrer}` : "";

  // Show the "no referrer" section when not connected or when connected but has no referrer
  const showNoReferrerSection = !isConnected || !hasReferrer;

  return (
    <div className={twMerge("w-full flex flex-col gap-6", showNoReferrerSection && "pb-[153px] lg:pb-0")}>
      <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-between">
        <div className="flex flex-col items-center lg:items-start">
          {isLoading ? (
            <Skeleton className="w-16 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatPercent(rebateRate) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.rebateRate"]()}</p>
        </div>
        <div className="flex flex-col items-center">
          {isLoading ? (
            <Skeleton className="w-24 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatUSD(totalRebates) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalRebates"]()}</p>
        </div>
        <div className="flex flex-col items-center lg:items-end">
          {isLoading ? (
            <Skeleton className="w-24 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatUSD(totalVolume) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">{m["referral.stats.totalTradingVolume"]()}</p>
        </div>
      </div>

      {isConnected && hasReferrer ? (
        <div className="w-full rounded-xl bg-surface-tertiary-gray px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <IconUser className="w-5 h-5 text-primitives-blue-light-400" />
            <p className="text-ink-primary-900 diatype-m-medium">{m["referral.stats.yourReferrer"]()}</p>
          </div>
          {isLoading ? (
            <Skeleton className="w-20 h-6" />
          ) : (
            <Badge text={referrerDisplay} color="blue" />
          )}
        </div>
      ) : (
        <>
          <div className="w-full h-px bg-outline-secondary-gray" />
          <div className="min-h-[280px] lg:min-h-[180px]">
            <div className="relative z-10 flex flex-col gap-8 lg:max-w-sm">
              <div className="flex flex-col gap-2">
                <h3 className="display-heading-xs text-ink-primary-900 max-w-sm">
                  {m["referral.traderSection.referTitle"]()}
                </h3>
                <p className="text-ink-tertiary-500 diatype-m-regular max-w-sm">
                  {m["referral.traderSection.referDescription"]({ percent: "15%" })}
                </p>
              </div>
              {isConnected ? (
                <Input
                  label={m["referral.traderSection.referralCodeLabel"]()}
                  value={referralCodeInput}
                  onChange={(e) => setReferralCodeInput(e.target.value)}
                  placeholder={m["referral.traderSection.referralCodePlaceholder"]()}
                  endContent={
                    <Button variant="link" className="p-0">
                      {m["referral.traderSection.submit"]()}
                    </Button>
                  }
                />
              ) : (
                <Button variant="primary" size="sm" onClick={() => showModal(Modals.Login)}>
                  {m["referral.affiliateSection.logIn"]()}
                </Button>
              )}
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
              {m["referral.affiliate"]()} <Badge text="Tier 1" color="rice" />
            </span>
          </Tab>
          <Tab title="trader">{m["referral.trader"]()}</Tab>
        </Tabs>
      </div>

      {mode === "affiliate" ? <AffiliateStats /> : <TraderStats />}
    </div>
  );
};

export type { ReferralMode };
