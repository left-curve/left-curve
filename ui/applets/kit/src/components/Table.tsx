import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type React from "react";

import { twMerge } from "#utils/twMerge.js";
import { tv, type VariantProps } from "tailwind-variants";

export type TableColumn<T> = ColumnDef<T>[];

interface TableProps<T> extends VariantProps<typeof tabsVariants> {
  bottomContent?: React.ReactNode;
  topContent?: React.ReactNode;
  columns: TableColumn<T>;
  data: T[];
}

export const Table = <T,>({ topContent, bottomContent, columns, data, style }: TableProps<T>) => {
  const table = useReactTable<T>({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
  });

  const styles = tabsVariants({
    style,
  });

  const { rows } = table.getRowModel();

  return (
    <div className={twMerge(styles.base(), rows.length ? "pb-2" : "pb-4")}>
      {topContent}
      <div
        className={twMerge(
          "scrollbar-none w-full min-w-fit whitespace-nowrap overflow-hidden relative overflow-x-scroll ",
        )}
      >
        {table.getHeaderGroups().map((headerGroup) => (
          <div
            key={headerGroup.id}
            style={{ gridTemplateColumns: `repeat(${columns.length}, 1fr)` }}
            className="grid w-full"
          >
            {headerGroup.headers.map((header) => {
              return (
                <div key={header.id} className={twMerge(styles.header(), "")}>
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </div>
              );
            })}
          </div>
        ))}

        {rows.map((row) => {
          const cells = row.getVisibleCells();
          return (
            <div
              key={row.id}
              style={{ gridTemplateColumns: `repeat(${columns.length}, 1fr)` }}
              className={twMerge(styles.row(), "grid w-full")}
            >
              {cells.map((cell) => {
                return (
                  <div key={cell.id} className={twMerge(styles.cell())}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
      {bottomContent}
    </div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "grid rounded-xl w-full gap-4 max-w-[calc(100vw-2rem)] overflow-x-scroll scrollbar-none",
    header: "whitespace-nowrap",
    cell: "min-w-fit",
    row: "",
  },
  variants: {
    style: {
      default: {
        base: "bg-rice-25 shadow-account-card px-4 pt-4",
        header:
          "p-4 last:text-end bg-green-bean-100 text-gray-500 first:rounded-l-xl diatype-xs-bold last:justify-end last:rounded-r-xl",
        cell: "px-4 py-2 diatype-sm-medium first:pl-4 last:pr-4 flex last:justify-end last:text-end",
        row: "border-b border-gray-100 last:border-b-0",
      },
      simple: {
        base: "text-gray-500",
        header: "p-2 text-gray-500 diatype-xs-regular flex last:justify-end",
        cell: "px-2 items-center flex last:justify-end diatype-xs-medium",
        row: "rounded-lg hover:bg-rice-50",
      },
    },
  },
  defaultVariants: {
    style: "default",
  },
});
