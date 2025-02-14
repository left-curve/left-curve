import type React from "react";
import { Badge } from "../Badge";
import { Button } from "../Button";
import { Tabs } from "../Tabs";
import { Table } from "./Table";

interface PoolTableProps {
  data: any[];
}

export const PoolTable: React.FC<PoolTableProps> = ({ data }) => {
  return (
    <Table
      data={data}
      columns={[
        {
          accessorKey: "vault",
          header: "Vault",
          cell: ({ cell, row }) => {
            return (
              <div className="flex gap-2 text-lg">
                <div className="flex">
                  <img
                    src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                    alt=""
                    className="h-6 min-w-6 rounded-full"
                  />
                  <img
                    src="https://w7.pngwing.com/pngs/268/1013/png-transparent-ethereum-eth-hd-logo-thumbnail.png"
                    alt=""
                    className="h-6 min-w-6 -ml-1 rounded-full"
                  />
                </div>
                <p className="min-w-fit">ETH-USD</p>
              </div>
            );
          },
        },
        {
          accessorKey: "type",
          header: "Type",
          cell: ({ cell, row }) => {
            return (
              <div className="flex items-center justify-end">
                <Badge text={cell.getValue() as string} color="green" />
              </div>
            );
          },
        },
        {
          accessorKey: "apr",
          header: "APR",
          cell: ({ cell }) => (
            <div className="flex items-center justify-end">{cell.getValue() as string}</div>
          ),
        },
        {
          accessorKey: "liquidity",
          header: "Liquidity Available",
          cell: ({ cell }) => (
            <div className="flex items-center justify-end">{cell.getValue() as string}</div>
          ),
        },
        {
          accessorKey: "tvl",
          header: "TVL",
          cell: ({ cell }) => (
            <div className="flex items-center justify-end">{cell.getValue() as string}</div>
          ),
        },
        {
          accessorKey: "risk",
          header: "Risk Level",
          cell: ({ cell }) => (
            <div className="flex items-center justify-end">{cell.getValue() as string}</div>
          ),
        },
      ]}
      topContent={<Tabs defaultKey="Tokens" keys={["Tokens", "Earn", "Pool"]} />}
      bottomContent={
        <Button variant="secondary" className="self-center">
          View All
        </Button>
      }
    />
  );
};
