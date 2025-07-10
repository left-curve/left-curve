import type { PropsWithChildren } from "react";

import { uid } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";

import { Cell, Table } from "@left-curve/applets-kit";
import { EmptyPlaceholder } from "../foundation/EmptyPlaceholder";
import { StrategyCard, createContext } from "@left-curve/applets-kit";
import { useAccount, useAppConfig, useBalances, useConfig } from "@left-curve/store";

import type { TableColumn } from "@left-curve/applets-kit";
import type { PairSymbols } from "@left-curve/dango/types";
import type { LpCoin, WithAmount } from "@left-curve/store/types";

type EarnProps = {
  navigate: (pair: PairSymbols) => void;
};

const [EarnProvider, useEarn] = createContext<EarnProps>({
  name: "EarnContext",
});

const EarnContainer: React.FC<PropsWithChildren<EarnProps>> = ({ children, navigate }) => {
  return <EarnProvider value={{ navigate }}>{children}</EarnProvider>;
};

const EarnHeader: React.FC = () => {
  return (
    <div className="flex flex-col items-center justify-center pb-6 text-center">
      <img
        src="/images/emojis/detailed/pig.svg"
        alt="pig-detailed"
        className="w-[148px] h-[148px]"
      />
      <h1 className="exposure-h1-italic text-gray-900">{m["earn.title"]()}</h1>
      <p className="text-tertiary-500 diatype-lg-medium">{m["earn.description"]()}</p>
    </div>
  );
};

const EarnPoolsCards: React.FC = () => {
  const { navigate } = useEarn();
  const { data: appConfig } = useAppConfig();

  return (
    <div className="flex gap-4 scrollbar-none justify-start lg:justify-between p-4 overflow-x-auto overflow-y-visible">
      {Object.values(appConfig?.pairs || {})
        .slice(0, 4)
        .map((pair, index) => (
          <StrategyCard
            key={uid()}
            pair={pair}
            index={index}
            onSelect={navigate}
            labels={{
              party: m["earn.party"](),
              earn: m["earn.earn"](),
              deposit: m["earn.deposit"](),
              select: m["earn.select"](),
              apy: m["earn.apy"](),
              tvl: m["earn.tvl"](),
            }}
          />
        ))}
    </div>
  );
};

const EarnUserPoolsTable: React.FC = () => {
  const { navigate } = useEarn();
  const { getCoinInfo } = useConfig();
  const { account } = useAccount();
  const { data: balances = {}, isLoading } = useBalances({ address: account?.address });

  const userPools = Object.entries(balances)
    .filter(([denom]) => denom.includes("dex"))
    .map(([denom, amount]) => {
      const coin = getCoinInfo(denom);
      return { ...coin, amount } as WithAmount<LpCoin>;
    });

  const columns: TableColumn<WithAmount<LpCoin>> = [
    {
      header: m["earn.vault"](),
      cell: ({ row }) => {
        return <Cell.Assets assets={[row.original.base, row.original.quote]} />;
      },
    },
    {
      header: m["earn.myPosition"](),
      cell: ({ row }) => <Cell.Text text={row.original.amount} />,
    },
    {
      header: m["earn.apr"](),
      cell: () => <Cell.Text text="-" />,
    },
    {
      header: m["earn.tvl"](),
      cell: ({ row }) => <Cell.Text text="-" />,
    },
    {
      id: "manage",
      header: () => {
        return <Cell.Text text={m["earn.manage"]()} className="text-right px-1" />;
      },
      cell: ({ row }) => (
        <Cell.Action
          classNames={{ cell: "items-end", button: "m-0 px-1" }}
          action={() =>
            navigate({
              baseSymbol: row.original.base.symbol,
              quoteSymbol: row.original.quote.symbol,
            })
          }
          label="Manage"
        />
      ),
    },
  ];

  return (
    <div className="flex w-full p-4">
      <Table
        data={userPools}
        columns={columns}
        isLoading={isLoading}
        emptyComponent={
          <EmptyPlaceholder component={m["earn.noLiquidity"]()} className="h-[7rem]" />
        }
      />
    </div>
  );
};

export const Earn = Object.assign(EarnContainer, {
  Header: EarnHeader,
  PoolsCards: EarnPoolsCards,
  UserPoolsTable: EarnUserPoolsTable,
});
