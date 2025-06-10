import { Cell, Table, type TableColumn, Tabs } from "@left-curve/applets-kit";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";
import { useState } from "react";
import { useApp } from "~/hooks/useApp";
import { Modals } from "../modals/RootModal";

export const OpenOrder: React.FC = () => {
  const [activeTab, setActiveTab] = useState<"open order" | "trade history">("open order");
  const { showModal } = useApp();

  const data = [
    {
      time: new Date(),
      type: "Limit",
      coin: {
        symbol: "USDC",
        name: "USDC",
        denom: "usdc",
        decimals: 6,
        type: "contract",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
      } as AnyCoin,
      direction: "Long",
      size: 0.063,
      orderValue: 11.98,
      price: 1.889,
      reduceOnly: false,
      triggerConditions: "N/A",
      onCancel: () => showModal(Modals.ProSwapCloseAll),
    },
    {
      time: new Date(),
      type: "Limit",
      coin: {
        symbol: "USDC",
        name: "USDC",
        denom: "usdc",
        decimals: 6,
        type: "contract",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
      } as AnyCoin,
      direction: "Long",
      size: 0.063,
      orderValue: 11.98,
      price: 1.889,
      reduceOnly: false,
      triggerConditions: "N/A",
      onCancel: () => showModal(Modals.ProSwapCloseAll),
    },
  ];

  const columns: TableColumn<{
    time: Date;
    type: string;
    coin: AnyCoin;
    direction: string;
    size: number;
    orderValue: number;
    price: number;
    reduceOnly: boolean;
    triggerConditions: string;
    onCancel: () => void;
  }> = [
    {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.time} />,
    },
    {
      header: "Type",
      cell: ({ row }) => <Cell.Text text={row.original.type} />,
    },
    {
      header: "Coin",
      cell: ({ row }) => <Cell.Asset noImage asset={row.original.coin} />,
    },
    {
      header: "Direction",
      cell: ({ row }) => <Cell.Text text={row.original.direction} />,
    },
    {
      header: "Size",
      cell: ({ row }) => <Cell.Text text={row.original.size} />,
    },
    {
      header: "Order Value",
      cell: ({ row }) => <Cell.Text text={row.original.orderValue} />,
    },
    {
      header: "Price",
      cell: ({ row }) => <Cell.Text text={row.original.price} />,
    },
    {
      header: "Reduce Only",
      cell: ({ row }) => <Cell.Text text={row.original.reduceOnly ? "Yes" : "No"} />,
    },
    {
      header: "Trigger Conditions",
      cell: ({ row }) => <Cell.Text text={row.original.triggerConditions} />,
    },
    {
      header: "Cancel All",
      cell: ({ row }) => (
        <Cell.Action action={row.original.onCancel} label="Cancel" className="items-end" />
      ),
    },
  ];

  return (
    <div className="flex-1 p-4 bg-rice-25 flex flex-col gap-2 shadow-card-shadow pb-20 lg:pb-0">
      <div className="relative">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeTab}
          keys={["open order", "trade history"]}
          onTabChange={(tab) => setActiveTab(tab as "open order" | "trade history")}
        />

        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[0.25rem]" />
      </div>
      <Table data={data} columns={columns} style="simple" />
    </div>
  );
};
