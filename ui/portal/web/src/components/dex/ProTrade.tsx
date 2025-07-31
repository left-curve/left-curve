import type { TableColumn } from "@left-curve/applets-kit";
import {
  AddressVisualizer,
  Badge,
  Cell,
  IconChevronDownFill,
  Table,
  Tabs,
  createContext,
  twMerge,
  useInputs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type { OrderId, OrdersByUserResponse, PairId } from "@left-curve/dango/types";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { useAppConfig, useConfig, usePrices, useProTradeState } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";
import type { PropsWithChildren } from "react";
import { useEffect, useState } from "react";
import { useApp } from "~/hooks/useApp";
import { m } from "~/paraglide/messages";
import { ChartIQ } from "../foundation/ChartIQ";
import { EmptyPlaceholder } from "../foundation/EmptyPlaceholder";
import { Modals } from "../modals/RootModal";
import { OrderBookOverview } from "./OrderBookOverview";
import { SearchToken } from "./SearchToken";
import { TradeButtons } from "./TradeButtons";
import { TradeMenu } from "./TradeMenu";

const [ProTradeProvider, useProTrade] = createContext<{
  state: ReturnType<typeof useProTradeState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "ProTradeContext",
});

type ProTradeProps = {
  action: "buy" | "sell";
  onChangeAction: (action: "buy" | "sell") => void;
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
  orderType: "limit" | "market";
  onChangeOrderType: (orderType: "limit" | "market") => void;
};

const ProTradeContainer: React.FC<PropsWithChildren<ProTradeProps>> = ({
  action,
  onChangeAction,
  pairId,
  onChangePairId,
  orderType,
  onChangeOrderType,
  children,
}) => {
  const controllers = useInputs();

  const state = useProTradeState({
    controllers,
    pairId,
    onChangePairId,
    action,
    onChangeAction,
    orderType,
    onChangeOrderType,
  });

  return <ProTradeProvider value={{ state, controllers }}>{children}</ProTradeProvider>;
};

const ProTradeHeader: React.FC = () => {
  const { data: config } = useAppConfig();
  const { isLg } = useMediaQuery();
  const [isExpanded, setIsExpanded] = useState(isLg);
  const { state } = useProTrade();
  const { pairId, onChangePairId } = state;
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const navigate = useNavigate();

  useEffect(() => {
    setIsExpanded(isLg);
  }, [isLg]);

  return (
    <div className="flex bg-surface-tertiary-rice lg:gap-8 p-4 flex-col lg:flex-row w-full lg:justify-between">
      <div className="flex gap-8 items-center justify-between lg:items-start w-full lg:w-auto">
        <div className="flex lg:flex-col gap-2">
          <SearchToken pairId={pairId} onChangePairId={onChangePairId} />
          <div className="lg:pl-8">
            <Badge text="Spot" color="blue" size="s" />
          </div>
        </div>
        <div className="flex gap-2 items-center">
          <div
            className="cursor-pointer flex items-center justify-center lg:hidden"
            onClick={() => setIsExpanded(!isExpanded)}
          >
            <IconChevronDownFill
              className={twMerge("text-tertiary-500 w-4 h-4 transition-all", {
                "rotate-180": isExpanded,
              })}
            />
          </div>
          {/*   <IconEmptyStar className="w-5 h-5 text-tertiary-500" /> */}
        </div>
      </div>
      <AnimatePresence initial={false}>
        {isExpanded ? (
          <motion.div
            layout="position"
            layoutId="protrade-header"
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: isLg ? 0 : 0.3, ease: "easeInOut" }}
            className="gap-2 lg:gap-5 grid grid-cols-1 lg:flex lg:flex-wrap lg:items-center overflow-hidden"
          >
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start pt-8 lg:pt-0">
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.spot.price"]()}
              </p>
              <p className="diatype-sm-bold text-secondary-700">
                {getPrice(1, pairId.baseDenom, { format: true })}
              </p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.spot.24hChange"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-tertiary-500">
                {m["dex.protrade.spot.volume"]()}
              </p>
              <p className="diatype-sm-bold w-full text-center">-</p>
            </div>
            <div className="items-center flex gap-1 flex-row lg:flex-col min-w-[4rem] lg:items-start">
              <p className="diatype-xs-medium text-tertiary-500">{m["dex.contract"]()}</p>
              <AddressVisualizer
                address={config?.addresses.dex || "0x"}
                withIcon
                onClick={(url) => navigate({ to: url })}
                classNames={{ text: "diatype-sm-bold" }}
              />
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
};

const ProTradeOverview: React.FC = () => {
  const { state } = useProTrade();
  return <OrderBookOverview state={state} />;
};

const ProTradeChart: React.FC = () => {
  const { state } = useProTrade();
  const { isLg } = useMediaQuery();
  const { baseCoin, quoteCoin } = state;

  if (!isLg) return null;

  return (
    <div className="shadow-account-card bg-surface-secondary-rice h-full">
      <ChartIQ coins={{ base: baseCoin, quote: quoteCoin }} />
    </div>
  );
};

const ProTradeMenu: React.FC = () => {
  const { isLg } = useMediaQuery();
  const { state, controllers } = useProTrade();
  return (
    <>
      <TradeMenu state={state} controllers={controllers} />
      {!isLg ? <TradeButtons state={state} /> : null}
    </>
  );
};

const ProTradeOrders: React.FC = () => {
  const { showModal, settings } = useApp();
  const { coins } = useConfig();
  const [activeTab, setActiveTab] = useState<"open order" | "trade history">("open order");

  const { state } = useProTrade();
  const { orders, baseCoin } = state;
  const { formatNumberOptions } = settings;

  const columns: TableColumn<OrdersByUserResponse & { id: OrderId }> = [
    /*  {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.time} />,
    }, */
    {
      header: m["dex.protrade.spot.ordersTable.id"](),
      cell: ({ row }) => {
        const orderId = Decimal(row.original.id);
        const value = orderId.gte("9223372036854775807")
          ? Decimal("18446744073709551615").minus(orderId).toString()
          : orderId.toString();
        return (
          <Cell.Text
            text={value}
            className="diatype-xs-regular text-secondary-700 hover:text-primary-900 cursor-pointer"
          />
        );
      },
    },

    {
      header: m["dex.protrade.spot.ordersTable.type"](),
      cell: (_) => <Cell.Text text="Limit" />,
    },
    {
      header: m["dex.protrade.spot.ordersTable.pair"](),
      cell: ({ row }) => {
        return (
          <div className="flex items-center gap-1">
            <Cell.PairName
              className="diatype-xs-medium"
              pairId={{
                baseDenom: row.original.baseDenom,
                quoteDenom: row.original.quoteDenom,
              }}
            />
          </div>
        );
      },
    },
    {
      header: m["dex.protrade.spot.ordersTable.direction"](),
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
      id: "remaining",
      header: () =>
        m["dex.protrade.spot.ordersTable.remaining"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={formatNumberOptions}
          value={formatNumber(
            Decimal(row.original.remaining)
              .div(Decimal(10).pow(coins.byDenom[row.original.baseDenom].decimals))
              .toFixed(),
            {
              ...formatNumberOptions,
              maxSignificantDigits: 10,
            },
          )}
        />
      ),
    },
    {
      id: "size",
      header: () =>
        m["dex.protrade.spot.ordersTable.size"]({
          symbol: baseCoin.symbol,
        }),
      cell: ({ row }) => (
        <Cell.Number
          formatOptions={formatNumberOptions}
          value={formatNumber(
            Decimal(row.original.amount)
              .div(Decimal(10).pow(coins.byDenom[row.original.baseDenom].decimals))
              .toFixed(),
            {
              ...formatNumberOptions,
              maxSignificantDigits: 10,
            },
          )}
        />
      ),
    },
    {
      header: m["dex.protrade.spot.price"](),
      cell: ({ row }) => (
        <Cell.Text
          text={formatNumber(
            Decimal(row.original.price)
              .times(
                Decimal(10).pow(
                  coins.byDenom[row.original.baseDenom].decimals -
                    coins.byDenom[row.original.quoteDenom].decimals,
                ),
              )
              .toFixed(),
            { ...formatNumberOptions, maxSignificantDigits: 10 },
          )}
        />
      ),
    },
    {
      id: "cancel-order",
      header: () => (
        <Cell.Action
          isDisabled={!orders.data.length}
          action={() =>
            showModal(Modals.ProTradeCloseAll, { ordersId: orders.data.map((o) => o.id) })
          }
          label={m["common.cancelAll"]()}
          classNames={{
            cell: "items-end diatype-xs-regular",
            button: "!exposure-xs-italic m-0 p-0 px-1 h-fit ",
          }}
        />
      ),
      cell: ({ row }) => (
        <Cell.Action
          action={() => showModal(Modals.ProTradeCloseOrder, { orderId: row.original.id })}
          label={m["common.cancel"]()}
          classNames={{
            cell: "items-end",
            button: "!exposure-xs-italic m-0 p-0 px-1 h-fit",
          }}
        />
      ),
    },
  ];

  return (
    <div className="flex-1 p-4 bg-surface-secondary-rice flex flex-col gap-2 shadow-account-card pb-20 lg:pb-5 z-10">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeTab}
          keys={["open order", "trade history"]}
          onTabChange={(tab) => setActiveTab(tab as "open order" | "trade history")}
          classNames={{ button: "exposure-xs-italic", base: "z-10" }}
        />

        <span className="w-full absolute h-[2px] bg-secondary-gray bottom-[0px] z-0" />
      </div>
      <div className="w-full h-full relative">
        {activeTab === "open order" ? (
          <Table
            data={orders.data}
            columns={columns}
            style="simple"
            classNames={{
              row: "h-fit",
              header: "pt-0",
              base: "pb-0",
              cell: twMerge("diatype-xs-regular py-1", {
                "group-hover:bg-transparent": !orders.data.length,
              }),
            }}
            emptyComponent={
              activeTab === "open order" ? (
                <EmptyPlaceholder
                  component={m["dex.protrade.spot.noOpenOrders"]()}
                  className="h-[3.5rem]"
                />
              ) : null
            }
          />
        ) : (
          <div className="min-h-[88.8px] w-full backdrop-blur-[8px] flex items-center justify-center exposure-l-italic text-primary-rice">
            {m["dex.protrade.underDevelopment"]()}
          </div>
        )}
      </div>
    </div>
  );
};

export const ProTrade = Object.assign(ProTradeContainer, {
  Header: ProTradeHeader,
  Chart: ProTradeChart,
  Orders: ProTradeOrders,
  OrderBook: ProTradeOverview,
  TradeMenu: ProTradeMenu,
});
