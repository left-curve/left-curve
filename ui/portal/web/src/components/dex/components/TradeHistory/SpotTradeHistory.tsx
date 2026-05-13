import { Cell, FormattedNumber, useApp } from "@left-curve/applets-kit";
import {
  useConfig,
  usePublicClient,
  useAccount,
  useQueryWithPagination,
  useTradeCoins,
} from "@left-curve/store";
import { calculateTradeSize, Decimal } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { TimeInForceOption, type Trade } from "@left-curve/dango/types";
import { TradeHistoryTable } from "./TradeHistoryTable";

import type { TableColumn } from "@left-curve/applets-kit";

export const SpotTradeHistory: React.FC = () => {
  const { settings } = useApp();
  const { coins } = useConfig();
  const { account } = useAccount();
  const publicClient = usePublicClient();

  const { baseCoin } = useTradeCoins();
  const { formatNumberOptions } = settings;

  const { data, pagination, isLoading } = useQueryWithPagination({
    enabled: !!account,
    queryKey: ["tradeHistory", account?.address as string],
    queryFn: async () => {
      if (!account) throw new Error();
      return await publicClient.queryTrades({ address: account.address });
    },
  });

  const columns: TableColumn<Trade> = [
    {
      header: m["dex.protrade.tradeHistory.pair"](),
      cell: ({ row }) => (
        <div className="flex items-center gap-1">
          <Cell.PairName
            className="diatype-xs-medium"
            pairId={{
              baseDenom: row.original.baseDenom,
              quoteDenom: row.original.quoteDenom,
            }}
          />
        </div>
      ),
    },
    {
      header: m["dex.protrade.tradeHistory.direction"](),
      cell: ({ row }) => (
        <Cell.OrderDirection
          text={m["dex.protrade.spot.direction"]({
            direction: row.original.direction,
          })}
          direction={row.original.direction}
        />
      ),
    },
    {
      header: m["dex.protrade.history.type"](),
      cell: ({ row }) => (
        <Cell.Text
          text={m["dex.protrade.orderType"]({
            orderType:
              row.original.timeInForce === TimeInForceOption.GoodTilCanceled ? "limit" : "market",
          })}
        />
      ),
    },
    {
      id: "size",
      header: () =>
        m["dex.protrade.history.size"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={formatNumberOptions}
          value={calculateTradeSize(
            row.original,
            coins.byDenom[row.original.baseDenom].decimals,
          ).toFixed()}
        />
      ),
    },
    {
      header: m["dex.protrade.history.price"](),
      cell: ({ row }) => (
        <Cell.Text
          text={
            <FormattedNumber
              number={Decimal(row.original.clearingPrice)
                .times(
                  Decimal(10).pow(
                    coins.byDenom[row.original.baseDenom].decimals -
                      coins.byDenom[row.original.quoteDenom].decimals,
                  ),
                )
                .toFixed()}
              as="span"
            />
          }
        />
      ),
    },
    {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.createdAt} dateFormat="MM/dd/yy h:mm a" />,
    },
  ];

  return (
    <TradeHistoryTable
      data={data}
      columns={columns}
      isLoading={isLoading}
    />
  );
};
