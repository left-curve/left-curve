import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import React from "react";

import { twMerge } from "#utils/twMerge.js";

export type TableColumn<T> = ColumnDef<T>[];

type TableProps<T> = {
  bottomContent?: React.ReactNode;
  topContent?: React.ReactNode;
  columns: TableColumn<T>;
  data: T[];
};

export const Table = <T,>({ topContent, bottomContent, columns, data }: TableProps<T>) => {
  const table = useReactTable<T>({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
  });

  const { rows } = table.getRowModel();

  return (
    <div
      className={twMerge(
        "bg-rice-25 shadow-account-card grid rounded-xl w-full px-4 pt-4 gap-4 overflow-hidden",
        rows.length ? "pb-2" : "pb-4",
      )}
    >
      {topContent}
      <div
        style={{ gridTemplateColumns: `repeat(${columns.length}, 1fr)` }}
        className={twMerge("overflow-y-auto scrollbar-none w-full grid relative")}
      >
        {table.getHeaderGroups().map((headerGroup) => (
          <React.Fragment key={headerGroup.id}>
            {headerGroup.headers.map((header, index) => {
              return (
                <div
                  key={header.id}
                  className="p-4 last:text-end bg-green-bean-100 text-gray-500 first:rounded-l-xl diatype-xs-bold"
                  style={
                    headerGroup.headers.length - 1 === index
                      ? { borderRadius: "0 16px 16px 0", textAlign: "end" }
                      : {}
                  }
                >
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </div>
              );
            })}
          </React.Fragment>
        ))}

        {rows.map((row, rowIndex) => {
          const cells = row.getVisibleCells();
          return (
            <React.Fragment key={row.id}>
              {cells.map((cell, index) => {
                return (
                  <div
                    key={cell.id}
                    style={{
                      paddingLeft: index === 0 ? "1rem" : undefined,
                      paddingRight: index === cells.length - 1 ? "1rem" : undefined,
                    }}
                    className={twMerge("px-4 py-2 diatype-sm-medium", {
                      "border-b border-gray-100": rowIndex !== rows.length - 1,
                    })}
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </div>
                );
              })}
            </React.Fragment>
          );
        })}
      </div>
      {bottomContent}
    </div>
  );
};
