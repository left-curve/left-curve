import { useConfig } from "@left-curve/store";

import { Button, useApp } from "@left-curve/applets-kit";
import { ButtonLink } from "../foundation/ButtonLink";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";

import type { Coins } from "@left-curve/dango/types";
import type React from "react";

interface Props {
  balances: Coins;
  showAllAssets?: () => void;
}

export const AssetsSection: React.FC<Props> = ({ balances, showAllAssets }) => {
  const { coins } = useConfig();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const sortedCoinsByBalance = Object.entries(coins.byDenom).sort(([denomA], [denomB]) => {
    const balanceA = BigInt(balances[denomA] || "0");
    const balanceB = BigInt(balances[denomB] || "0");
    return balanceB > balanceA ? 1 : -1;
  });

  return (
    <div className="flex-col bg-surface-secondary-rice shadow-account-card lg:flex rounded-xl p-4 gap-2 w-full h-full  min-h-[10rem] lg:justify-between">
      <div className="flex items-center justify-between w-full">
        <p className="h4-bold text-primary-900">{m["common.assets"]()}</p>
        {showAllAssets ? (
          <Button variant="link" size="xs" onClick={showAllAssets}>
            {m["common.viewAll"]()}
          </Button>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-4 items-center justify-between">
        {sortedCoinsByBalance.map(([denom, coin]) => {
          const amount = balances[denom];
          if (denom === "dango") return null;
          return (
            <div className="flex gap-2 items-center" key={`preview-asset-${denom}`}>
              <img src={coin.logoURI} alt={coin.symbol} className="h-7 w-7 drag-none select-none" />
              <div className="flex flex-col text-xs">
                <p>{coin.symbol}</p>
                <p className="text-tertiary-500">
                  {amount
                    ? formatNumber(
                        formatUnits(amount, coins.byDenom[denom].decimals),
                        formatNumberOptions,
                      )
                    : "0"}
                </p>
              </div>
            </div>
          );
        })}
      </div>
      <div className="lg:self-end gap-4 items-center justify-center w-full lg:max-w-[256px] hidden lg:flex lg:mt-1">
        <ButtonLink fullWidth size="md" to="/transfer" search={{ action: "receive" }}>
          {m["common.fund"]()}
        </ButtonLink>
        <ButtonLink
          fullWidth
          variant="secondary"
          size="md"
          to="/transfer"
          search={{ action: "send" }}
        >
          {m["common.send"]()}
        </ButtonLink>
      </div>
    </div>
  );
};
