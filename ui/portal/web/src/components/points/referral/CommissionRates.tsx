import { Cell, Table } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import type React from "react";

type CommissionTier = {
  tier: string;
  tradingVolume: string;
  thirtyDayReferralVolume: string;
  commission: string;
};

const mockCommissionTiers: CommissionTier[] = [
  {
    tier: "Tier 1",
    tradingVolume: "~$10,000",
    thirtyDayReferralVolume: "0",
    commission: "10%",
  },
  {
    tier: "Tier 2",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$10,000,000",
    commission: "15%",
  },
  {
    tier: "Tier 3",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$50,000,000",
    commission: "20%",
  },
  {
    tier: "Tier 4",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$80,000,000",
    commission: "25%",
  },
  {
    tier: "Tier 5",
    tradingVolume: "0",
    thirtyDayReferralVolume: "~$100,000,000",
    commission: "50%",
  },
];

export const CommissionRates: React.FC = () => {
  const columns: TableColumn<CommissionTier> = [
    {
      header: "Tier",
      cell: ({ row }) => (
        <Cell.Text className="text-ink-primary-900 diatype-m-medium" text={row.original.tier} />
      ),
    },
    {
      header: "Trading Volume",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.tradingVolume}
        />
      ),
    },
    {
      header: "30-day Referral Volume",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-medium"
          text={row.original.thirtyDayReferralVolume}
        />
      ),
    },
    {
      header: "Commission",
      cell: ({ row }) => (
        <Cell.Text
          className="text-ink-primary-900 diatype-m-bold"
          text={row.original.commission}
        />
      ),
    },
  ];

  return (
    <div className="w-full flex flex-col gap-4">
      <h3 className="exposure-m-italic text-ink-primary-900">Commission Rates</h3>
      <Table
        data={mockCommissionTiers}
        columns={columns}
        style="simple"
        classNames={{
          base: "rounded-xl bg-surface-primary-rice border border-outline-secondary-gray overflow-hidden",
          header:
            "bg-surface-tertiary-rice text-ink-tertiary-500 diatype-s-medium border-b border-outline-secondary-gray",
          cell: "px-4 py-3",
          row: "border-b border-outline-secondary-gray last:border-b-0",
        }}
      />
    </div>
  );
};
