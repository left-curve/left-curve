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
import { twMerge } from "../../../utils";

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

  return (
    <div className="bg-rice-25 shadow-card-shadow grid rounded-3xl w-full p-4 gap-4 overflow-hidden">
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

        {table.getRowModel().rows.map((row) => {
          return (
            <React.Fragment key={row.id}>
              {row.getVisibleCells().map((cell) => {
                return (
                  <div key={cell.id} className="px-6 py-4 last:text-end diatype-sm-medium">
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
