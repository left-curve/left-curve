import { Cell, Skeleton, Table } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import { useReferralConfig } from "@left-curve/store";
import type React from "react";
import { useMemo } from "react";

type CommissionTier = {
  tier: string;
  tradingVolume: string;
  thirtyDayReferralVolume: string;
  commission: string;
};

/**
 * Format a volume value for display
 * Converts from microunits to human readable format
 */
const formatVolume = (value: string): string => {
  const num = Number(value);
  if (Number.isNaN(num) || num === 0) return "0";

  // Convert from microunits (1e-6) to base units
  const baseValue = num / 1_000_000;

  if (baseValue >= 1_000_000_000) {
    return `~$${(baseValue / 1_000_000_000).toFixed(0)}B`;
  }
  if (baseValue >= 1_000_000) {
    return `~$${(baseValue / 1_000_000).toFixed(0)}M`;
  }
  if (baseValue >= 1_000) {
    return `~$${(baseValue / 1_000).toFixed(0)}K`;
  }
  return `~$${baseValue.toLocaleString()}`;
};

/**
 * Format a percentage (e.g., "0.2" -> "20%")
 */
const formatPercent = (value: string): string => {
  const num = Number(value);
  if (Number.isNaN(num)) return "0%";
  return `${(num * 100).toFixed(0)}%`;
};

// Default tiers to show while loading or if config is not available
const DEFAULT_TIERS: CommissionTier[] = [
  {
    tier: "Tier 1",
    tradingVolume: "~$10,000",
    thirtyDayReferralVolume: "0",
    commission: "10%",
  },
  {
    tier: "Tier 2",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$10M",
    commission: "20%",
  },
  {
    tier: "Tier 3",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$100M",
    commission: "30%",
  },
  {
    tier: "Tier 4",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$1B",
    commission: "40%",
  },
];

export const CommissionRates: React.FC = () => {
  const { config, isLoading } = useReferralConfig();

  // Transform config tiers to display format
  const commissionTiers = useMemo<CommissionTier[]>(() => {
    if (!config?.tiers || config.tiers.length === 0) {
      return DEFAULT_TIERS;
    }

    // Create Tier 1 with default commission (no volume requirement)
    const tiers: CommissionTier[] = [
      {
        tier: "Tier 1",
        tradingVolume: "~$10,000",
        thirtyDayReferralVolume: "0",
        commission: formatPercent(config.default_commission_rebound),
      },
    ];

    // Add tiers from config
    config.tiers.forEach((tier, index) => {
      tiers.push({
        tier: `Tier ${index + 2}`,
        tradingVolume: "0",
        thirtyDayReferralVolume: formatVolume(tier.min_volume),
        commission: formatPercent(tier.commission_rebound),
      });
    });

    return tiers;
  }, [config]);

  const columns: TableColumn<CommissionTier> = [
    {
      header: "Tier",
      cell: ({ row }) => <Cell.Text text={row.original.tier} />,
    },
    {
      header: "Trading Volume",
      cell: ({ row }) => <Cell.Text text={row.original.tradingVolume} />,
    },
    {
      header: "30-day Referral Volume",
      cell: ({ row }) => <Cell.Text text={row.original.thirtyDayReferralVolume} />,
    },
    {
      header: "Commission",
      cell: ({ row }) => <Cell.Text text={row.original.commission} />,
    },
  ];

  if (isLoading) {
    return (
      <div className="w-full flex flex-col gap-4">
        <h3 className="exposure-m-italic text-ink-primary-900">Commission Rates</h3>
        <div className="space-y-3">
          {[...Array(4)].map((_, i) => (
            <Skeleton key={i} className="w-full h-12" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="w-full flex flex-col gap-4">
      <h3 className="exposure-m-italic text-ink-primary-900">Commission Rates</h3>
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
