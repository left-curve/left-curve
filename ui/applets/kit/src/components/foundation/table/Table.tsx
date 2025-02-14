import {
  type ColumnDef,
  PaginationState,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type React from "react";

interface TableProps<T = any> {
  bottomContent?: React.ReactNode;
  topContent?: React.ReactNode;
  columns: ColumnDef<T>[];
  data: T[];
}

export const Table: React.FC<TableProps> = ({ topContent, bottomContent, columns, data }) => {
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
  });

  return (
    <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-3xl w-full p-4 gap-4">
      {topContent}
      <div className="overflow-y-auto scrollbar-none w-full">
        <table className="table-auto w-full">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id} className=" text-[#717680] font-semibold text-xs">
                {headerGroup.headers.map((header) => {
                  return (
                    <th
                      key={header.id}
                      colSpan={header.colSpan}
                      className="text-end p-4 bg-green-bean-100 first:text-start first:rounded-l-xl last:rounded-r-xl"
                    >
                      {flexRender(header.column.columnDef.header, header.getContext())}
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => {
              return (
                <tr key={row.id} className="p-4 border-b border-b-gray-100">
                  {row.getVisibleCells().map((cell) => {
                    return (
                      <td key={cell.id} className="p-4">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    );
                  })}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      {bottomContent}
    </div>
  );
};
