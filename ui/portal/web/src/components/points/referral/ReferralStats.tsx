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
  useReferralParams,
  useSetReferral,
  getReferralCode,
  getReferralLink,
} from "@left-curve/store";
import type React from "react";
import { useMemo, useState } from "react";

type ReferralMode = "affiliate" | "trader";

type ReferralStatsProps = {
  mode: ReferralMode;
  onModeChange: (mode: ReferralMode) => void;
};

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

const formatPercent = (value: string | undefined): string => {
  if (!value) return "0%";
  const num = Number(value);
  if (Number.isNaN(num)) return "0%";
  return `${(num * 100).toFixed(0)}%`;
};

const truncateUrl = (url: string, maxLength = 20): string => {
  if (url.length <= maxLength) return url;
  const start = url.slice(0, maxLength - 5);
  return `${start}...`;
};

const COMMISSION_LOOKBACK_SECONDS = 30 * 24 * 60 * 60;

/**
 * Given sorted tier thresholds and the current 30-day rolling referees volume,
 * return the current tier number (0 = base, 1 = first tier, etc.)
 * and the next tier's volume threshold (or null if at max tier).
 */
function resolveTier(
  sortedThresholds: number[],
  rollingRefereesVolume: number,
): { currentTier: number; nextTierVolume: number | null } {
  let currentTier = 0;
  for (let i = 0; i < sortedThresholds.length; i++) {
    if (rollingRefereesVolume >= sortedThresholds[i]) {
      currentTier = i + 1;
    } else {
      return { currentTier, nextTierVolume: sortedThresholds[i] };
    }
  }
  return { currentTier, nextTierVolume: null };
}

type AffiliateLockedBannerProps = {
  isConnected: boolean;
  onLogin: () => void;
  onTrade: () => void;
};

const AffiliateLockedBanner: React.FC<AffiliateLockedBannerProps> = ({
  isConnected,
  onLogin,
  onTrade,
}) => (
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
        <Button variant="primary" onClick={onTrade}>
          {m["referral.affiliateSection.tradeNow"]()}
        </Button>
      ) : (
        <Button variant="primary" onClick={onLogin}>
          {m["referral.affiliateSection.logIn"]()}
        </Button>
      )}
    </div>
    <img
      src="/images/points/referral-banner.svg"
      alt="Referral banner"
      className="absolute bottom-0 right-1/2 translate-x-1/2 lg:right-[3rem] lg:translate-x-0 w-[200px] lg:w-auto h-auto object-contain pointer-events-none"
    />
  </div>
);

const AffiliateCredentialsLoading: React.FC = () => (
  <div className="flex flex-col lg:flex-row gap-4">
    <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center gap-4">
      <Skeleton className="h-5 w-28" />
      <Skeleton className="h-5 w-28" />
    </div>
    <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center gap-4">
      <Skeleton className="h-5 w-28" />
      <Skeleton className="h-5 w-20" />
    </div>
  </div>
);

export const AffiliateStats: React.FC = () => {
  const { showModal, navigate } = useApp();
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  const { referralData, isLoading: dataLoading } = useReferralData({
    userIndex,
  });

  // 30-day lookback for rolling referees volume (used for tier progression).
  const since30d = useMemo(() => Math.floor(Date.now() / 1000) - COMMISSION_LOOKBACK_SECONDS, []);
  const { referralData: referralData30d, isLoading: data30dLoading } = useReferralData({
    userIndex,
    since: since30d,
    enabled: isConnected,
  });

  const { settings, isLoading: settingsLoading } = useReferralSettings({
    userIndex,
  });
  const { referralParams, isLoading: paramsLoading } = useReferralParams();

  const minReferrerVolume = Number(referralParams?.minReferrerVolume ?? "10000");
  const currentVolume = Number(referralData?.volume ?? "0");
  const isTierOneEligible = isConnected && currentVolume >= minReferrerVolume;

  const isLoading =
    isConnected &&
    (dataLoading || settingsLoading || paramsLoading || (isTierOneEligible && data30dLoading));

  // Sorted tier thresholds from referral params.
  const sortedThresholds = useMemo(() => {
    const tiers = referralParams?.referrerCommissionRates.tiers;
    if (!tiers) return [];
    return Object.keys(tiers)
      .map((v) => Number(v))
      .filter((v) => !Number.isNaN(v))
      .sort((a, b) => a - b);
  }, [referralParams]);

  // 30-day rolling referees volume for tier calculation.
  const rollingRefereesVolume = Number(referralData30d?.refereesVolume ?? "0");

  const { currentTier, nextTierVolume } = useMemo(
    () => resolveTier(sortedThresholds, rollingRefereesVolume),
    [sortedThresholds, rollingRefereesVolume],
  );

  // Progress bar: pre-Tier 1 = lifetime volume toward minReferrerVolume,
  // post-Tier 1 = 30-day rolling referees volume toward next tier.
  const targetVolume = isTierOneEligible ? nextTierVolume : minReferrerVolume;
  const progressValue = isTierOneEligible ? rollingRefereesVolume : currentVolume;
  const progress =
    isConnected && targetVolume
      ? Math.min((progressValue / targetVolume) * 100, 100)
      : isConnected && isTierOneEligible && !targetVolume
        ? 100
        : 0;
  const remaining = targetVolume ? Math.max(targetVolume - progressValue, 0) : 0;

  const referralCode = getReferralCode(userIndex);
  const referralLink = getReferralLink(userIndex);
  const truncatedLink = truncateUrl(referralLink);

  const commissionRate = settings?.commissionRate ?? "0";
  const shareRatio = settings?.shareRatio ?? "0";

  const rateDisplay = isConnected
    ? `${formatPercent(commissionRate)} / ${formatPercent(shareRatio)}`
    : "-- / --";

  const totalCommission = referralData?.commissionEarnedFromReferees ?? "0";
  const totalRefereesVolume = referralData?.refereesVolume ?? "0";
  const totalReferees = referralData?.refereeCount ?? 0;
  const activeReferees = referralData?.cumulativeActiveReferees ?? 0;

  // Tier 1 label key = "Tier 1", next tier label = "Tier N+1".
  const tierLabel = `Tier ${currentTier + 1}`;

  const progressLeftLabel = isConnected
    ? isTierOneEligible
      ? nextTierVolume
        ? m["referral.stats.volumeUntilNextTier"]({ amount: formatUSD(remaining), tier: tierLabel })
        : m["referral.stats.maxTierReached"]()
      : m["referral.stats.volumeUntilTier1"]({ amount: formatUSD(remaining) })
    : m["referral.stats.notLoggedIn"]();
  const progressRightLabel = targetVolume ? `$${(targetVolume / 1000).toFixed(0)}K` : "";

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
            <p className="text-ink-tertiary-500 diatype-m-medium">
              {m["referral.stats.commissionRate"]()}
            </p>
          </div>
          <div className="flex flex-col items-center">
            {isLoading ? (
              <Skeleton className="w-24 h-8" />
            ) : (
              <p className="text-ink-primary-900 h3-bold">
                {isConnected ? formatUSD(totalCommission) : "--"}
              </p>
            )}
            <p className="text-ink-tertiary-500 diatype-m-medium">
              {m["referral.stats.totalCommission"]()}
            </p>
          </div>
          <div className="flex flex-col items-center lg:items-end">
            {isLoading ? (
              <Skeleton className="w-24 h-8" />
            ) : (
              <p className="text-primitives-warning-500 h3-bold">
                {isConnected ? formatUSD(totalRefereesVolume) : "--"}
              </p>
            )}
            <p className="text-ink-tertiary-500 diatype-m-medium">
              {m["referral.stats.totalReferralVolume"]()}
            </p>
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
              <p className="text-ink-tertiary-500 diatype-m-medium">
                {m["referral.stats.totalReferees"]()}
              </p>
              {isLoading ? (
                <Skeleton className="w-12 h-6" />
              ) : (
                <p className="text-ink-primary-900 diatype-m-bold">
                  {isConnected ? totalReferees : "--"}
                </p>
              )}
            </div>
            <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
              <p className="text-ink-tertiary-500 diatype-m-medium">
                {m["referral.stats.totalActiveReferees"]()}
              </p>
              {isLoading ? (
                <Skeleton className="w-12 h-6" />
              ) : (
                <p className="text-ink-primary-900 diatype-m-bold">
                  {isConnected ? activeReferees : "--"}
                </p>
              )}
            </div>
          </div>

          {isLoading ? (
            <AffiliateCredentialsLoading />
          ) : isTierOneEligible ? (
            <div className="flex flex-col lg:flex-row gap-4">
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">
                  {m["referral.stats.myReferralLink"]()}
                </p>
                <div className="flex items-center gap-2">
                  <p className="text-ink-primary-900 diatype-m-bold">{truncatedLink}</p>
                  <TextCopy
                    copyText={referralLink}
                    className="w-4 h-4 cursor-pointer text-ink-tertiary-500"
                  />
                </div>
              </div>
              <div className="flex-1 bg-surface-primary-gray shadow-account-card rounded-xl px-4 py-3 flex justify-between items-center">
                <p className="text-ink-tertiary-500 diatype-m-medium">
                  {m["referral.stats.myReferralCode"]()}
                </p>
                <div className="flex items-center gap-2">
                  <p className="text-ink-primary-900 diatype-m-bold">{referralCode}</p>
                  <TextCopy copyText={referralCode} className="w-5 h-5 text-ink-tertiary-500" />
                </div>
              </div>
            </div>
          ) : (
            <AffiliateLockedBanner
              isConnected={isConnected}
              onTrade={() => navigate("/trade")}
              onLogin={() => showModal(Modals.Authenticate, { action: "signin" })}
            />
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

  const {
    referrer,
    hasReferrer,
    isLoading: referrerLoading,
  } = useReferrer({
    userIndex,
  });
  const { referralData, isLoading: dataLoading } = useReferralData({
    userIndex,
  });
  const { settings, isLoading: settingsLoading } = useReferralSettings({
    userIndex: referrer ?? undefined,
    enabled: hasReferrer,
  });

  const { mutate: submitSetReferral, isPending: isSubmitting } = useSetReferral({
    onSuccess: () => setReferralCodeInput(""),
  });

  const isLoading = isConnected && (referrerLoading || dataLoading || settingsLoading);

  const rebateRate = settings?.shareRatio ?? "0";
  const totalRebates = referralData?.commissionSharedByReferrer ?? "0";
  const totalVolume = Number(referralData?.volume ?? "0");

  const referrerDisplay = referrer ? `#${referrer}` : "";
  const showNoReferrerSection = !isConnected || !hasReferrer;

  const handleSubmitReferralCode = () => {
    const referrerIndex = Number(referralCodeInput);
    if (!userIndex || Number.isNaN(referrerIndex) || referrerIndex <= 0) return;
    submitSetReferral({ referrer: referrerIndex, referee: userIndex });
  };

  return (
    <div
      className={twMerge(
        "w-full flex flex-col gap-6",
        showNoReferrerSection && "pb-[153px] lg:pb-0",
      )}
    >
      <div className="flex flex-col gap-4 items-center lg:flex-row lg:justify-between">
        <div className="flex flex-col items-center lg:items-start">
          {isLoading ? (
            <Skeleton className="w-16 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatPercent(rebateRate) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["referral.stats.rebateRate"]()}
          </p>
        </div>
        <div className="flex flex-col items-center">
          {isLoading ? (
            <Skeleton className="w-24 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatUSD(totalRebates) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["referral.stats.totalRebates"]()}
          </p>
        </div>
        <div className="flex flex-col items-center lg:items-end">
          {isLoading ? (
            <Skeleton className="w-24 h-8" />
          ) : (
            <p className="text-utility-warning-600 h3-bold">
              {isConnected ? formatUSD(totalVolume) : "--"}
            </p>
          )}
          <p className="text-ink-tertiary-500 diatype-m-medium">
            {m["referral.stats.totalTradingVolume"]()}
          </p>
        </div>
      </div>

      {isConnected && hasReferrer ? (
        <div className="w-full rounded-xl bg-surface-tertiary-gray px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <IconUser className="w-5 h-5 text-primitives-blue-light-400" />
            <p className="text-ink-primary-900 diatype-m-medium">
              {m["referral.stats.yourReferrer"]()}
            </p>
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
                    <Button
                      variant="link"
                      className="p-0"
                      onClick={handleSubmitReferralCode}
                      disabled={isSubmitting || !referralCodeInput}
                    >
                      {isSubmitting
                        ? m["referral.submitting"]()
                        : m["referral.traderSection.submit"]()}
                    </Button>
                  }
                />
              ) : (
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => showModal(Modals.Authenticate, { action: "signin" })}
                >
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
  const { account, isConnected } = useAccount();
  const userIndex = account?.index;

  const { referralData } = useReferralData({ userIndex });
  const { referralParams } = useReferralParams();

  const since30d = useMemo(() => Math.floor(Date.now() / 1000) - COMMISSION_LOOKBACK_SECONDS, []);
  const { referralData: referralData30d } = useReferralData({
    userIndex,
    since: since30d,
    enabled: isConnected,
  });

  const minReferrerVolume = Number(referralParams?.minReferrerVolume ?? "10000");
  const currentVolume = Number(referralData?.volume ?? "0");
  const isTierOneEligible = isConnected && currentVolume >= minReferrerVolume;

  const sortedThresholds = useMemo(() => {
    const tiers = referralParams?.referrerCommissionRates.tiers;
    if (!tiers) return [];
    return Object.keys(tiers)
      .map((v) => Number(v))
      .filter((v) => !Number.isNaN(v))
      .sort((a, b) => a - b);
  }, [referralParams]);

  const rollingRefereesVolume = Number(referralData30d?.refereesVolume ?? "0");
  const { currentTier } = useMemo(
    () => resolveTier(sortedThresholds, rollingRefereesVolume),
    [sortedThresholds, rollingRefereesVolume],
  );

  // Tier 0 = not yet eligible; Tier 1+ = base + higher tiers.
  const tierBadgeText = isTierOneEligible ? `Tier ${currentTier + 1}` : null;

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
              {m["referral.affiliate"]()}{" "}
              {tierBadgeText && <Badge text={tierBadgeText} color="rice" />}
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
