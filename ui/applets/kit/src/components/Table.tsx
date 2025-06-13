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
  classNames?: {
    base?: string;
    header?: string;
    cell?: string;
    row?: string;
  };
}

export const Table = <T,>({
  topContent,
  bottomContent,
  columns,
  data,
  style,
  classNames,
}: TableProps<T>) => {
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
    <div className={twMerge(styles.base(), rows.length ? "pb-2" : "pb-4", classNames?.base)}>
      {topContent}
      <table
        className={twMerge(
          "scrollbar-none w-full min-w-fit whitespace-nowrap overflow-hidden relative overflow-x-scroll ",
        )}
      >
        {table.getHeaderGroups().map((headerGroup) => (
          <thead key={headerGroup.id}>
            {headerGroup.headers.map((header) => {
              return (
                <td key={header.id} className={twMerge(styles.header(), "", classNames?.header)}>
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </td>
              );
            })}
          </thead>
        ))}

        <tbody>
          {rows.map((row) => {
            const cells = row.getVisibleCells();
            return (
              <tr key={row.id} className={twMerge(styles.row(), classNames?.row)}>
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
        base: "bg-bg-secondary-rice shadow-account-card px-4 pt-4",
        header:
          "p-4 last:text-end bg-green-bean-100 text-tertiary-500 first:rounded-l-xl diatype-xs-bold last:justify-end last:rounded-r-xl",
        cell: "px-4 py-2 diatype-sm-medium first:pl-4 last:pr-4 last:justify-end last:text-end",
        row: "border-b border-gray-100 last:border-b-0",
      },
      simple: {
        base: "text-tertiary-500 border-separate",
        header: "p-2 text-tertiary-500 diatype-xs-regular last:text-end",
        cell: "px-2 last:text-end diatype-xs-medium first:rounded-l-xl last:rounded-r-xl group-hover:bg-bg-tertiary-rice",
        row: "rounded-xl group",
      },
    },
  },
  defaultVariants: {
    style: "default",
  },
});
