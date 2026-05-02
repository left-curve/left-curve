import { type ReactNode, createContext, use } from "react";
import { View, Text, type ViewProps } from "react-native";
import { twMerge } from "@left-curve/foundation";

// ---------------------------------------------------------------------------
// Context: column widths are declared in Header, consumed by Row/Cell
// ---------------------------------------------------------------------------

type TableContextValue = {
  readonly columns: readonly string[];
};

const TableContext = createContext<TableContextValue>({ columns: [] });

// ---------------------------------------------------------------------------
// Table (root container)
// ---------------------------------------------------------------------------

export type TableProps = ViewProps & {
  children: ReactNode;
};

function TableRoot({ className, children, ...props }: TableProps) {
  return (
    <View className={twMerge("flex flex-col w-full", className)} {...props}>
      {children}
    </View>
  );
}

// ---------------------------------------------------------------------------
// Table.Header
// ---------------------------------------------------------------------------

export type TableHeaderProps = ViewProps & {
  /** Tailwind width classes for each column, e.g. ["w-[140px]","flex-1"] */
  columns: readonly string[];
  children: ReactNode;
};

function TableHeader({ columns, className, children, ...props }: TableHeaderProps) {
  return (
    <TableContext value={{ columns }}>
      <View
        className={twMerge(
          "flex flex-row items-center h-9 px-4",
          "border-b border-border-subtle",
          className,
        )}
        {...props}
      >
        {children}
      </View>
    </TableContext>
  );
}

// ---------------------------------------------------------------------------
// Table.HeaderCell
// ---------------------------------------------------------------------------

export type TableHeaderCellProps = ViewProps & {
  /** 0-based column index — used to look up the width from Header's columns */
  index: number;
  children?: ReactNode;
};

function TableHeaderCell({ index, className, children, ...props }: TableHeaderCellProps) {
  const { columns } = use(TableContext);
  const widthClass = columns[index] ?? "";

  return (
    <View className={twMerge(widthClass, "pr-3", className)} {...props}>
      <Text className="text-fg-tertiary text-[10px] font-medium uppercase tracking-wide">
        {children}
      </Text>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Table.Row
// ---------------------------------------------------------------------------

export type TableRowProps = ViewProps & {
  /** Column widths — must match the Header's columns */
  columns: readonly string[];
  /** When true the row shows a hover highlight */
  hoverable?: boolean;
  children: ReactNode;
};

function TableRow({ columns, hoverable = true, className, children, ...props }: TableRowProps) {
  return (
    <TableContext value={{ columns }}>
      <View
        className={twMerge(
          "flex flex-row items-center h-12 px-4",
          "border-b border-border-subtle",
          hoverable && "hover:bg-bg-tint transition-[background] duration-150 ease-[var(--ease)]",
          className,
        )}
        {...props}
      >
        {children}
      </View>
    </TableContext>
  );
}

// ---------------------------------------------------------------------------
// Table.Cell
// ---------------------------------------------------------------------------

export type TableCellProps = ViewProps & {
  /** 0-based column index */
  index: number;
  children: ReactNode;
};

function TableCell({ index, className, children, ...props }: TableCellProps) {
  const { columns } = use(TableContext);
  const widthClass = columns[index] ?? "";

  return (
    <View className={twMerge(widthClass, "justify-center pr-3", className)} {...props}>
      {children}
    </View>
  );
}

// ---------------------------------------------------------------------------
// Table.Empty
// ---------------------------------------------------------------------------

export type TableEmptyProps = ViewProps & {
  children: ReactNode;
};

function TableEmpty({ className, children, ...props }: TableEmptyProps) {
  return (
    <View className={twMerge("py-10 items-center justify-center", className)} {...props}>
      <Text className="text-fg-tertiary text-[13px]">{children}</Text>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Compound export
// ---------------------------------------------------------------------------

export const Table = Object.assign(TableRoot, {
  Header: TableHeader,
  HeaderCell: TableHeaderCell,
  Row: TableRow,
  Cell: TableCell,
  Empty: TableEmpty,
});
