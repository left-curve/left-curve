import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";

import { tv } from "tailwind-variants";
import { twMerge } from "#utils/twMerge.js";

import type { ColumnDef, ColumnFiltersState, Row, Updater } from "@tanstack/react-table";
import type React from "react";
import type { VariantProps } from "tailwind-variants";

export type TableColumn<T> = ColumnDef<T>[];
export type { ColumnFiltersState };

export type TableClassNames = {
  base?: string;
  header?: string;
  cell?: string;
  row?: string;
};

interface TableProps<T> extends VariantProps<typeof tabsVariants> {
  bottomContent?: React.ReactNode;
  topContent?: React.ReactNode;
  columns: TableColumn<T>;
  data: T[];
  columnFilters?: ColumnFiltersState;
  onColumnFiltersChange?: (updater: Updater<ColumnFiltersState>) => void;
  onRowClick?: (row: Row<T>) => void;
  classNames?: TableClassNames;
}

export const Table = <T,>({
  topContent,
  bottomContent,
  columns,
  data,
  style,
  classNames,
  columnFilters,
  onColumnFiltersChange,
  onRowClick,
}: TableProps<T>) => {
  const table = useReactTable<T>({
    data,
    columns,
    state: { columnFilters },
    enableFilters: true,
    onColumnFiltersChange,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
  });

  const styles = tabsVariants({
    style,
  });

  const { rows } = table.getRowModel();

  return (
    <div className={twMerge(styles.base(), rows.length ? "pb-2" : "pb-4", classNames?.base)}>
      {topContent}
      <table
        className={twMerge(
          "scrollbar-none w-full min-w-fit whitespace-nowrap overflow-hidden relative overflow-x-scroll ",
        )}
      >
        <thead>
          {table.getHeaderGroups().map((headerGroup) => (
            <tr key={headerGroup.id}>
              {headerGroup.headers.map((header) => {
                return (
                  <td key={header.id} className={twMerge(styles.header(), classNames?.header)}>
                    {flexRender(header.column.columnDef.header, header.getContext())}
                  </td>
                );
              })}
            </tr>
          ))}
        </thead>

        <tbody>
          {rows.map((row) => {
            const cells = row.getVisibleCells();
            return (
              <tr
                key={row.id}
                className={twMerge(styles.row(), classNames?.row, { "cursor-pointer": onRowClick })}
                onClick={() => onRowClick?.(row)}
              >
                {cells.map((cell) => {
                  return (
                    <td key={cell.id} className={twMerge(styles.cell(), classNames?.cell)}>
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
      {bottomContent}
    </div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "grid rounded-xl w-full gap-4 max-w-[calc(100vw-2rem)] overflow-x-scroll scrollbar-none",
    header: "whitespace-nowrap",
    cell: "",
    row: "",
  },
  variants: {
    style: {
      default: {
        base: "bg-rice-25 shadow-account-card px-4 pt-4",
        header:
          "p-4 last:text-end bg-green-bean-100 text-gray-500 first:rounded-l-xl diatype-xs-bold last:justify-end last:rounded-r-xl text-start",
        cell: "px-4 py-2 diatype-sm-medium first:pl-4 last:pr-4 last:justify-end last:text-end text-start",
        row: "border-b border-gray-100 last:border-b-0",
      },
      simple: {
        base: "text-gray-500 border-separate",
        header: "p-2 text-gray-500 diatype-xs-regular last:text-end text-start",
        cell: "px-2 last:text-end diatype-xs-medium first:rounded-l-xl last:rounded-r-xl group-hover:bg-rice-50",
        row: "rounded-xl group",
      },
    },
  },
  defaultVariants: {
    style: "default",
  },
});
