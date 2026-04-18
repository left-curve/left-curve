import { forwardRef, useMemo } from "react";

import { Cell, IconButton, IconClose, Table, useApp } from "@left-curve/applets-kit";
import type { TableColumn } from "@left-curve/applets-kit";
import { formatNumber } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAppConfig, useFeeRateOverride } from "@left-curve/store";

type FeeTierRow = {
  tier: string;
  volume: string;
  taker: string;
  maker: string;
};

export const FeeTiers = forwardRef((_props, _ref) => {
  const { hideModal, settings } = useApp();
  const { formatNumberOptions } = settings;
  const { data: appConfig } = useAppConfig();
  const { override: feeRateOverride } = useFeeRateOverride();

  const rows = useMemo<FeeTierRow[]>(() => {
    const takerSchedule = appConfig?.perpsParam?.takerFeeRates;
    const makerSchedule = appConfig?.perpsParam?.makerFeeRates;
    if (!takerSchedule || !makerSchedule) return [];

    const takerTiers = Object.entries(takerSchedule.tiers).sort(
      ([a], [b]) => Number(a) - Number(b),
    );
    const makerTiers = Object.entries(makerSchedule.tiers).sort(
      ([a], [b]) => Number(a) - Number(b),
    );

    const allThresholds = [
      ...new Set([
        ...takerTiers.map(([t]) => t),
        ...makerTiers.map(([t]) => t),
      ]),
    ].sort((a, b) => Number(a) - Number(b));

    const takerMap = Object.fromEntries(takerTiers);
    const makerMap = Object.fromEntries(makerTiers);

    const result: FeeTierRow[] = [
      {
        tier: "0",
        volume: "--",
        taker: `${(Number(takerSchedule.base) * 100).toFixed(3)}%`,
        maker: `${(Number(makerSchedule.base) * 100).toFixed(3)}%`,
      },
    ];

    let lastTaker = takerSchedule.base;
    let lastMaker = makerSchedule.base;

    allThresholds.forEach((threshold, i) => {
      if (takerMap[threshold]) lastTaker = takerMap[threshold];
      if (makerMap[threshold]) lastMaker = makerMap[threshold];

      result.push({
        tier: String(i + 1),
        volume: formatNumber(threshold, { ...formatNumberOptions, currency: "USD" }),
        taker: `${(Number(lastTaker) * 100).toFixed(3)}%`,
        maker: `${(Number(lastMaker) * 100).toFixed(3)}%`,
      });
    });

    return result;
  }, [appConfig?.perpsParam, formatNumberOptions]);

  const columns: TableColumn<FeeTierRow> = [
    {
      header: m["dex.feeTiers.tier"](),
      cell: ({ row }) => <Cell.Text text={row.original.tier} />,
    },
    {
      header: m["dex.feeTiers.volume14d"](),
      cell: ({ row }) => <Cell.Text text={row.original.volume} />,
    },
    {
      header: m["dex.feeTiers.perpsTaker"](),
      cell: ({ row }) => <Cell.Text text={row.original.taker} />,
    },
    {
      header: m["dex.feeTiers.perpsMaker"](),
      cell: ({ row }) => <Cell.Text text={row.original.maker} />,
    },
  ];

  return (
    <div className="flex flex-col gap-4 bg-surface-primary-rice md:border border-outline-secondary-gray rounded-xl relative px-6 py-6 md:w-[32rem]">
      <IconButton
        className="absolute right-4 top-4"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>

      <div className="flex flex-col gap-2">
        <h2 className="h3-bold text-ink-primary-900">{m["dex.feeTiers.title"]()}</h2>
        <p className="text-ink-tertiary-500 diatype-sm-regular">
          {m["dex.feeTiers.description"]()}
        </p>
      </div>

      {feeRateOverride ? (
        <div className="flex flex-col gap-1 rounded-lg bg-surface-secondary-rice p-3">
          <p className="diatype-sm-medium text-ink-primary-900">
            {m["dex.feeTiers.customRate"]()}
          </p>
          <p className="diatype-xs-regular text-ink-tertiary-500">
            {m["dex.feeTiers.customRateDescription"]()}
          </p>
          <div className="flex items-center gap-4 mt-1">
            <p className="diatype-xs-medium text-ink-secondary-700">
              {m["dex.feeTiers.perpsTaker"]()}: {(Number(feeRateOverride.takerFeeRate) * 100).toFixed(3)}%
            </p>
            <p className="diatype-xs-medium text-ink-secondary-700">
              {m["dex.feeTiers.perpsMaker"]()}: {(Number(feeRateOverride.makerFeeRate) * 100).toFixed(3)}%
            </p>
          </div>
        </div>
      ) : null}

      <Table
        data={rows}
        columns={columns}
        classNames={{
          base: "p-0 bg-transparent shadow-none",
        }}
      />
    </div>
  );
});
