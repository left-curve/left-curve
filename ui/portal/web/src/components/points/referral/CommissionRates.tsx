import { Cell, Skeleton, Table, useApp } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useReferralParams } from "@left-curve/store";
import type React from "react";
import { useMemo } from "react";

type CommissionTier = {
  tier: string;
  tradingVolume: string;
  thirtyDayReferralVolume: string;
  commission: string;
};

const formatPercent = (value: string): string => {
  const num = Number(value);
  if (Number.isNaN(num)) return "0%";
  return `${(num * 100).toFixed(0)}%`;
};

const getCommissionTiers = (
  params: NonNullable<ReturnType<typeof useReferralParams>["referralParams"]>,
  formatVolume: (value: string) => string,
): CommissionTier[] => {
  const { minReferrerVolume, referrerCommissionRates } = params;
  const { base, tiers } = referrerCommissionRates;
  const tierEntries = Object.entries(tiers).sort(([a], [b]) => Number(a) - Number(b));
  const tierOneRow: CommissionTier = {
    tier: "Tier 1",
    tradingVolume: formatVolume(minReferrerVolume),
    thirtyDayReferralVolume: "0",
    commission: formatPercent(base),
  };

  if (tierEntries.length === 0) {
    return [tierOneRow];
  }

  return [
    tierOneRow,
    ...tierEntries.map(([minVolume, rate], index) => ({
      tier: `Tier ${index + 2}`,
      tradingVolume: "0",
      thirtyDayReferralVolume: formatVolume(minVolume),
      commission: formatPercent(rate),
    })),
  ];
};

const EMPTY_TIERS: CommissionTier[] = [];

export const CommissionRates: React.FC = () => {
  const { isConnected } = useAccount();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { referralParams, isLoading } = useReferralParams();

  const commissionTiers = useMemo<CommissionTier[]>(() => {
    if (!referralParams) {
      return EMPTY_TIERS;
    }

    const formatVolume = (value: string) =>
      formatNumber(value, { ...formatNumberOptions, currency: "USD" });

    return getCommissionTiers(referralParams, formatVolume);
  }, [referralParams, formatNumberOptions]);

  const columns: TableColumn<CommissionTier> = [
    {
      header: m["referral.commission.columns.tier"](),
      cell: ({ row }) => <Cell.Text text={row.original.tier} />,
    },
    {
      header: m["referral.commission.columns.tradingVolume"](),
      cell: ({ row }) => <Cell.Text text={row.original.tradingVolume} />,
    },
    {
      header: m["referral.commission.columns.thirtyDayReferralVolume"](),
      cell: ({ row }) => <Cell.Text text={row.original.thirtyDayReferralVolume} />,
    },
    {
      header: m["referral.commission.columns.commission"](),
      cell: ({ row }) => <Cell.Text text={row.original.commission} />,
    },
  ];

  if (!isConnected) return null;

  if (isLoading) {
    return (
      <div className="w-full flex flex-col gap-4">
        <h3 className="exposure-m-italic text-ink-primary-900">
          {m["referral.commission.title"]()}
        </h3>
        <div className="space-y-3">
          {[...Array(4)].map((_, i) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: "Only used for skeleton loading state"
            <Skeleton key={i} className="w-full h-12" />
          ))}
        </div>
      </div>
    );
  }

  if (commissionTiers.length === 0) return null;

  return (
    <div className="w-full flex flex-col gap-4">
      <h3 className="exposure-m-italic text-ink-primary-900">{m["referral.commission.title"]()}</h3>
      <Table
        data={commissionTiers}
        columns={columns}
        classNames={{
          base: "p-0 bg-transparent shadow-none",
        }}
      />
    </div>
  );
};
