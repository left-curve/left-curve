import { Cell, Table } from "@left-curve/applets-kit";
import { useConfig, usePrices } from "@left-curve/store";
import { useApp } from "~/hooks/useApp";

import type { TableColumn } from "@left-curve/applets-kit";
import type { Coins } from "@left-curve/dango/types";
import type { AnyCoin, WithAmount, WithPrice } from "@left-curve/store/types";

export type AssetsTableProps = {
  balances: Coins;
};

export const AssetsTable: React.FC<AssetsTableProps> = ({ balances }) => {
  const { coins: allCoins, state } = useConfig();
  const { settings } = useApp();
  const { getPrice } = usePrices();
  const { formatNumberOptions } = settings;

  const coins = allCoins[state.chainId];

  const data = Object.entries(balances).map(([denom, amount]) => {
    const coin = coins[denom];
    const price = getPrice(amount, denom, { format: true, formatOptions: formatNumberOptions });

    return { ...coin, price, amount };
  });

  const columns: TableColumn<WithAmount<WithPrice<AnyCoin>>> = [
    {
      header: "Asset",
      cell: ({ row }) => <Cell.Asset asset={row.original} />,
    },
    {
      header: "Market Price",
      cell: ({ row }) => (
        <Cell.MarketPrice denom={row.original.denom} formatOptions={formatNumberOptions} />
      ),
    },
    {
      header: "Available",
      cell: ({ row }) => (
        <Cell.Amount
          amount={row.original.amount}
          price={row.original.price}
          decimals={row.original.decimals}
        />
      ),
    },
    {
      header: "Total",
      cell: ({ row }) => (
        <Cell.Amount
          className="text-end"
          amount={row.original.amount}
          price={row.original.price}
          decimals={row.original.decimals}
        />
      ),
    },
  ];

  return <Table data={data} columns={columns} />;
};
