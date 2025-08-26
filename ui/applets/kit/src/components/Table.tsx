import {
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";

import { tv } from "tailwind-variants";
import { twMerge } from "../utils/twMerge.js";

import { Fragment } from "react";
import { Skeleton } from "./Skeleton";

import type React from "react";
import type { ColumnDef, ColumnFiltersState, Row, Updater } from "@tanstack/react-table";
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
  isLoading?: boolean;
  skeletonType?: "row" | "cell";
  emptyComponent?: React.ReactNode;
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
  isLoading = false,
  emptyComponent,
  skeletonType = "row",
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

  const showemptyComponent = !isLoading && rows.length === 0 && emptyComponent;
  const showTableRows = !isLoading && rows.length > 0;
  const showSkeleton = isLoading;

  return (
    <div className={twMerge(styles.base(), rows.length ? "pb-2" : "pb-4", classNames?.base)}>
      {topContent}
      <table
        className={twMerge(
          "scrollbar-none w-full min-w-fit whitespace-nowrap overflow-hidden relative overflow-x-scroll ",
        )}
      >
        <thead className="sticky top-0 bg-surface-secondary-rice z-10">
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
          {showSkeleton &&
            Array.from({ length: 3 }).map((_, rowIndex) => (
              <Fragment
                key={`row-skeleton-${
                  // biome-ignore lint/suspicious/noArrayIndexKey: Skeleton are not dynamic
                  rowIndex
                }`}
              >
                {skeletonType === "row" ? (
                  <tr className={twMerge(styles.row(), classNames?.row)}>
                    <td colSpan={columns.length} className={twMerge(styles.cell(), "pt-2")}>
                      <Skeleton className={twMerge("h-8 w-full", classNames?.row, styles.row())} />
                    </td>
                  </tr>
                ) : (
                  <tr className={twMerge(styles.row(), classNames?.row)}>
                    {columns.map((_, columnIndex) => (
                      <td
                        key={`column-skeleton-${
                          // biome-ignore lint/suspicious/noArrayIndexKey: Skeleton are not dynamic
                          columnIndex
                        }`}
                        className={twMerge(styles.cell(), classNames?.cell)}
                      >
                        <Skeleton className="h-8 w-full" />
                      </td>
                    ))}
                  </tr>
                )}
              </Fragment>
            ))}

          {showTableRows &&
            rows.map((row) => {
              const cells = row.getVisibleCells();
              return (
                <tr
                  key={`td-${row.id}`}
                  className={twMerge(styles.row(), classNames?.row, {
                    "cursor-pointer": onRowClick,
                  })}
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
      {showemptyComponent && <div className="w-full text-center">{emptyComponent}</div>}
      {bottomContent}
    </div>
  );
};

const tabsVariants = tv({
  slots: {
    base: "grid rounded-xl w-full max-w-[calc(100vw-2rem)] overflow-x-scroll scrollbar-none",
    header: "whitespace-nowrap",
    cell: "",
    row: "",
  },
  variants: {
    style: {
      default: {
        base: "bg-surface-secondary-rice shadow-account-card px-4 pt-4 gap-4",
        header:
          "p-4 last:text-end bg-secondary-green text-tertiary-500 first:rounded-l-xl diatype-xs-bold last:justify-end last:rounded-r-xl text-start",
        cell: "px-4 py-2 diatype-sm-medium first:pl-4 last:pr-4 last:justify-end last:text-end text-start",
        row: "border-b border-secondary-gray last:border-b-0",
      },
      simple: {
        base: "text-tertiary-500 border-separate gap-2",
        header: "p-2 text-tertiary-500 diatype-xs-regular last:text-end text-start",
        cell: "px-2 last:text-end diatype-xs-medium first:rounded-l-xl last:rounded-r-xl group-hover:bg-surface-tertiary-rice",
        row: "rounded-xl group",
      },
    },
  },
  defaultVariants: {
    style: "default",
  },
});
